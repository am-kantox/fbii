use crate::formats::encoding::decode_bytes;
use crate::formats::model::{
    Block, Book, BookFormat, Inline, ListItem, Metadata, TableCell, TableRow, TocItem,
};
use crate::utils::{AppError, Result};
use roxmltree::Node;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub fn parse_epub(path: &Path) -> Result<Book> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Parse(format!("Failed to open EPUB archive: {}", e)))?;

    // 1. Read META-INF/container.xml to find root OPF path
    let opf_path = get_container_opf_path(&mut archive)?;
    let opf_dir = Path::new(&opf_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // 2. Read OPF content
    let opf_bytes = read_zip_file(&mut archive, &opf_path)?;
    let opf_str = decode_bytes(&opf_bytes)?;
    let opf_doc = roxmltree::Document::parse(&opf_str)
        .map_err(|e| AppError::Parse(format!("Failed to parse OPF XML: {}", e)))?;

    let root = opf_doc.root_element();

    let mut metadata = Metadata {
        format: BookFormat::Epub,
        ..Default::default()
    };

    let mut manifest: HashMap<String, (String, String)> = HashMap::new(); // id -> (href, media-type)
    let mut spine: Vec<String> = Vec::new(); // itemref idrefs
    let mut ncx_id = None;
    let mut _nav_href = None;
    let mut cover_id = None;

    // Parse OPF sections
    for child in root.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "metadata" => {
                parse_epub_metadata(child, &mut metadata, &mut cover_id);
            }
            "manifest" => {
                for item in child
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "item")
                {
                    if let (Some(id), Some(href)) = (item.attribute("id"), item.attribute("href")) {
                        let media_type =
                            item.attribute("media-type").unwrap_or_default().to_string();
                        let properties = item.attribute("properties").unwrap_or_default();

                        let full_href = if opf_dir.is_empty() {
                            href.to_string()
                        } else {
                            format!("{}/{}", opf_dir, href)
                        };

                        manifest.insert(id.to_string(), (full_href.clone(), media_type.clone()));

                        if properties.contains("nav") {
                            _nav_href = Some(full_href.clone());
                        }
                        if media_type == "application/x-dtbncx+xml" {
                            ncx_id = Some(id.to_string());
                        }
                    }
                }
            }
            "spine" => {
                if let Some(toc_attr) = child.attribute("toc") {
                    ncx_id = Some(toc_attr.to_string());
                }
                for itemref in child
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "itemref")
                {
                    if let Some(idref) = itemref.attribute("idref") {
                        spine.push(idref.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // Set cover image key
    if let Some(cid) = cover_id {
        if let Some((href, _)) = manifest.get(&cid) {
            metadata.cover_image_key = Some(href.clone());
        }
    }

    // Load resources (images) into memory map
    let mut resources = HashMap::new();
    for (href, media_type) in manifest.values() {
        if media_type.starts_with("image/") {
            if let Ok(img_bytes) = read_zip_file(&mut archive, href) {
                resources.insert(href.clone(), img_bytes);
            }
        }
    }

    // Extract Table of Contents
    let mut toc = Vec::new();
    if let Some((ncx_href, _)) = ncx_id.as_ref().and_then(|id| manifest.get(id)) {
        let ncx_bytes = read_zip_file(&mut archive, ncx_href)?;
        let ncx_str = decode_bytes(&ncx_bytes)?;
        parse_ncx_toc(&ncx_str, &mut toc)?;
    }

    // Parse spine items into content blocks
    let mut content = Vec::new();
    for idref in spine {
        if let Some((href, _)) = manifest.get(&idref) {
            if let Ok(chapter_bytes) = read_zip_file(&mut archive, href) {
                if let Ok(chapter_str) = decode_bytes(&chapter_bytes) {
                    parse_xhtml_chapter(&chapter_str, &mut content);
                }
            }
        }
    }

    let file_stem = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "epub".to_string());
    let id = md5_hash(file_stem.as_bytes());

    let mut book = Book::new(id, path.to_string_lossy().to_string(), metadata);
    book.content = content;
    book.toc = toc;
    book.resources = resources;

    Ok(book)
}

fn get_container_opf_path(archive: &mut ZipArchive<std::fs::File>) -> Result<String> {
    let container_bytes = read_zip_file(archive, "META-INF/container.xml")?;
    let container_str = decode_bytes(&container_bytes)?;
    let doc = roxmltree::Document::parse(&container_str)
        .map_err(|e| AppError::Parse(format!("Invalid container.xml: {}", e)))?;

    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "rootfile")
    {
        if let Some(full_path) = node.attribute("full-path") {
            return Ok(full_path.to_string());
        }
    }

    Err(AppError::Parse(
        "Could not find full-path in container.xml".to_string(),
    ))
}

fn read_zip_file(archive: &mut ZipArchive<std::fs::File>, path: &str) -> Result<Vec<u8>> {
    let normalized = path.trim_start_matches('/');
    if let Ok(mut file) = archive.by_name(normalized) {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        return Ok(buf);
    }
    if let Ok(mut file) = archive.by_name(path) {
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        return Ok(buf);
    }
    Err(AppError::Parse(format!(
        "File '{}' not found in zip archive",
        path
    )))
}

fn parse_epub_metadata(node: Node, metadata: &mut Metadata, cover_id: &mut Option<String>) {
    let mut authors = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "title" => {
                if let Some(t) = child.text() {
                    metadata.title = t.trim().to_string();
                }
            }
            "creator" => {
                if let Some(a) = child.text() {
                    authors.push(a.trim().to_string());
                }
            }
            "subject" => {
                if let Some(g) = child.text() {
                    metadata.genres.push(g.trim().to_string());
                }
            }
            "description" => {
                if let Some(d) = child.text() {
                    metadata.annotation = Some(d.trim().to_string());
                }
            }
            "meta" => {
                if child.attribute("name") == Some("cover") {
                    if let Some(content) = child.attribute("content") {
                        *cover_id = Some(content.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    if !authors.is_empty() {
        metadata.authors = authors;
    }
}

fn parse_ncx_toc(ncx_xml: &str, toc: &mut Vec<TocItem>) -> Result<()> {
    let doc = roxmltree::Document::parse(ncx_xml)
        .map_err(|e| AppError::Parse(format!("NCX XML error: {}", e)))?;

    for node in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("navPoint"))
    {
        let mut title = String::new();
        let mut target_href = String::new();

        for child in node.children().filter(|n| n.is_element()) {
            let cname = child.tag_name().name();
            if cname.eq_ignore_ascii_case("navLabel") {
                if let Some(txt_node) = child
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("text"))
                {
                    if let Some(t) = txt_node.text() {
                        title = t.trim().to_string();
                    }
                }
            } else if cname.eq_ignore_ascii_case("content") {
                if let Some(src) = child.attribute("src") {
                    target_href = src.to_string();
                }
            }
        }

        if !title.is_empty() {
            toc.push(TocItem {
                title,
                target_href,
                block_index: 0,
                children: Vec::new(),
            });
        }
    }

    Ok(())
}

fn parse_xhtml_chapter(xhtml_str: &str, content: &mut Vec<Block>) {
    // Strip DOCTYPE declarations if present for robust roxmltree parsing
    let clean_str = if let Some(idx) = xhtml_str.find("<!DOCTYPE") {
        if let Some(end_idx) = xhtml_str[idx..].find('>') {
            let mut s = String::new();
            s.push_str(&xhtml_str[..idx]);
            s.push_str(&xhtml_str[idx + end_idx + 1..]);
            s
        } else {
            xhtml_str.to_string()
        }
    } else {
        xhtml_str.to_string()
    };

    match roxmltree::Document::parse(&clean_str) {
        Ok(doc) => {
            let root = doc.root_element();
            let body = root
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("body"))
                .unwrap_or(root);
            parse_html_nodes(body, content);
        }
        Err(e) => {
            eprintln!("XHTML parse error: {}", e);
        }
    }
}

fn parse_html_nodes(node: Node, content: &mut Vec<Block>) {
    for child in node.children().filter(|n| n.is_element()) {
        let name = child.tag_name().name().to_lowercase();
        match name.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name[1..].parse::<u8>().unwrap_or(1);
                let mut inlines = Vec::new();
                parse_html_inlines(child, &mut inlines);
                content.push(Block::Heading { level, inlines });
            }
            "p" | "div" => {
                let mut inlines = Vec::new();
                parse_html_inlines(child, &mut inlines);
                if !inlines.is_empty() {
                    content.push(Block::Paragraph(inlines));
                }
            }
            "blockquote" => {
                let mut blocks = Vec::new();
                parse_html_nodes(child, &mut blocks);
                content.push(Block::Quote(blocks));
            }
            "ul" | "ol" => {
                let ordered = name == "ol";
                let mut items = Vec::new();
                for li in child
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name().to_lowercase() == "li")
                {
                    let mut inlines = Vec::new();
                    parse_html_inlines(li, &mut inlines);
                    items.push(ListItem { inlines });
                }
                content.push(Block::List { ordered, items });
            }
            "table" => {
                let mut rows = Vec::new();
                for tr in child
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name().to_lowercase() == "tr")
                {
                    let mut cells = Vec::new();
                    for cell in tr.children().filter(|n| n.is_element()) {
                        let cname = cell.tag_name().name().to_lowercase();
                        if cname == "td" || cname == "th" {
                            let is_header = cname == "th";
                            let mut inlines = Vec::new();
                            parse_html_inlines(cell, &mut inlines);
                            cells.push(TableCell { is_header, inlines });
                        }
                    }
                    rows.push(TableRow { cells });
                }
                content.push(Block::Table { rows });
            }
            "img" => {
                if let Some(src) = child.attribute("src") {
                    let alt = child.attribute("alt").map(|s| s.to_string());
                    content.push(Block::Image {
                        key: src.to_string(),
                        alt,
                    });
                }
            }
            _ => {
                parse_html_nodes(child, content);
            }
        }
    }
}

fn parse_html_inlines(node: Node, out: &mut Vec<Inline>) {
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                if !t.is_empty() {
                    out.push(Inline::Text(t.to_string()));
                }
            }
        } else if child.is_element() {
            let name = child.tag_name().name().to_lowercase();
            match name.as_str() {
                "b" | "strong" => {
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    out.push(Inline::Bold(inner));
                }
                "i" | "em" => {
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    out.push(Inline::Italic(inner));
                }
                "u" => {
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    out.push(Inline::Underline(inner));
                }
                "s" | "strike" | "del" => {
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    out.push(Inline::Strike(inner));
                }
                "code" => {
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    let txt = inner
                        .iter()
                        .map(|i| i.plain_text())
                        .collect::<Vec<_>>()
                        .join("");
                    out.push(Inline::Code(txt));
                }
                "a" => {
                    let target = child.attribute("href").unwrap_or_default().to_string();
                    let mut inner = Vec::new();
                    parse_html_inlines(child, &mut inner);
                    out.push(Inline::Link {
                        target,
                        inlines: inner,
                    });
                }
                "br" => {
                    out.push(Inline::LineBreak);
                }
                "img" => {
                    if let Some(src) = child.attribute("src") {
                        let alt = child.attribute("alt").map(|s| s.to_string());
                        out.push(Inline::Image {
                            key: src.to_string(),
                            alt,
                        });
                    }
                }
                _ => {
                    parse_html_inlines(child, out);
                }
            }
        }
    }
}

fn md5_hash(bytes: &[u8]) -> String {
    use sha1::Digest;
    let hash = sha1::Sha1::digest(bytes);
    format!("{:x}", hash)
}
