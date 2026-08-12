use crate::formats::encoding::decode_bytes;
use crate::formats::model::{
    Block, Book, BookFormat, Inline, Metadata, PoemStanza, TableCell, TableRow, TocItem,
};
use crate::utils::{AppError, Result};
use roxmltree::Node;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub fn parse_fb2_bytes(bytes: &[u8], file_path: &str) -> Result<Book> {
    let xml_str = decode_bytes(bytes)?;
    let doc = roxmltree::Document::parse(&xml_str)
        .map_err(|e| AppError::Parse(format!("FB2 XML parse error: {}", e)))?;

    let root = doc.root_element();

    let mut metadata = Metadata {
        format: BookFormat::Fb2,
        ..Default::default()
    };

    let mut resources = HashMap::new();

    // Extract binary objects (images)
    for node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "binary")
    {
        if let Some(id) = node.attribute("id") {
            let key = id.trim_start_matches('#').to_string();
            let base64_text: String = node
                .text()
                .unwrap_or_default()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if let Ok(decoded) = base64_decode(&base64_text) {
                resources.insert(key, decoded);
            }
        }
    }

    // Extract description & metadata
    if let Some(desc_node) = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "description")
    {
        if let Some(title_info) = desc_node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "title-info")
        {
            parse_title_info(title_info, &mut metadata);
        }
    }

    let mut content = Vec::new();
    let mut toc = Vec::new();

    // Parse body elements
    for body_node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "body")
    {
        parse_body(body_node, &mut content, &mut toc);
    }

    let id = crate::formats::book_id_for_path(Path::new(file_path));

    let mut book = Book::new(id, file_path, metadata);
    book.content = content;
    book.toc = toc;
    book.resources = resources;

    Ok(book)
}

pub fn parse_fb2_zip(path: &Path) -> Result<Book> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Parse(format!("Failed to open ZIP archive: {}", e)))?;

    for i in 0..archive.len() {
        let mut zip_file = archive
            .by_index(i)
            .map_err(|e| AppError::Parse(format!("Zip read error: {}", e)))?;
        let name = zip_file.name().to_lowercase();
        if name.ends_with(".fb2") || name.ends_with(".xml") {
            let mut buffer = Vec::new();
            zip_file.read_to_end(&mut buffer)?;
            let mut book = parse_fb2_bytes(&buffer, &path.to_string_lossy())?;
            book.metadata.format = BookFormat::Fb2Zip;
            return Ok(book);
        }
    }

    Err(AppError::Parse(
        "No .fb2 file found inside ZIP archive".to_string(),
    ))
}

fn parse_title_info(node: Node, metadata: &mut Metadata) {
    let mut authors = Vec::new();

    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "book-title" => {
                if let Some(t) = child.text() {
                    metadata.title = t.trim().to_string();
                }
            }
            "author" => {
                let mut name_parts = Vec::new();
                for author_child in child.children().filter(|n| n.is_element()) {
                    if let Some(t) = author_child.text() {
                        let t = t.trim();
                        if !t.is_empty() {
                            name_parts.push(t.to_string());
                        }
                    }
                }
                if !name_parts.is_empty() {
                    authors.push(name_parts.join(" "));
                }
            }
            "sequence" => {
                if let Some(name) = child.attribute("name") {
                    metadata.series_name = Some(name.to_string());
                }
                if let Some(num_str) = child.attribute("number") {
                    if let Ok(num) = num_str.parse::<u32>() {
                        metadata.series_index = Some(num);
                    }
                }
            }
            "genre" => {
                if let Some(g) = child.text() {
                    metadata.genres.push(g.trim().to_string());
                }
            }
            "annotation" => {
                let mut inlines = Vec::new();
                parse_inlines(child, &mut inlines);
                let text = inlines
                    .iter()
                    .map(|i| i.plain_text())
                    .collect::<Vec<_>>()
                    .join("");
                if !text.is_empty() {
                    metadata.annotation = Some(text);
                }
            }
            "coverpage" => {
                if let Some(img_node) = child
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "image")
                {
                    if let Some(href) = img_node
                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                        .or_else(|| img_node.attribute("href"))
                    {
                        metadata.cover_image_key = Some(href.trim_start_matches('#').to_string());
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

fn parse_body(node: Node, content: &mut Vec<Block>, toc: &mut Vec<TocItem>) {
    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "section" => {
                parse_section(child, content, toc);
            }
            "title" => {
                let block_idx = content.len();
                let mut inlines = Vec::new();
                parse_inlines(child, &mut inlines);
                let title_text = inlines
                    .iter()
                    .map(|i| i.plain_text())
                    .collect::<Vec<_>>()
                    .join("");
                content.push(Block::Heading { level: 1, inlines });
                toc.push(TocItem {
                    title: title_text,
                    target_href: format!("#block-{}", block_idx),
                    block_index: block_idx,
                    children: Vec::new(),
                });
            }
            _ => {
                if let Some(block) = parse_block_element(child) {
                    content.push(block);
                }
            }
        }
    }
}

fn parse_section(node: Node, content: &mut Vec<Block>, toc: &mut Vec<TocItem>) {
    let mut section_toc_children = Vec::new();
    let section_start_idx = content.len();
    let mut section_title = None;

    for child in node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "title" => {
                let _block_idx = content.len();
                let mut inlines = Vec::new();
                parse_inlines(child, &mut inlines);
                let t_text = inlines
                    .iter()
                    .map(|i| i.plain_text())
                    .collect::<Vec<_>>()
                    .join("");
                section_title = Some(t_text);
                content.push(Block::Heading { level: 2, inlines });
            }
            "section" => {
                parse_section(child, content, &mut section_toc_children);
            }
            _ => {
                if let Some(block) = parse_block_element(child) {
                    content.push(block);
                }
            }
        }
    }

    if let Some(title) = section_title {
        toc.push(TocItem {
            title,
            target_href: format!("#block-{}", section_start_idx),
            block_index: section_start_idx,
            children: section_toc_children,
        });
    } else if !section_toc_children.is_empty() {
        toc.extend(section_toc_children);
    }
}

fn parse_block_element(node: Node) -> Option<Block> {
    match node.tag_name().name() {
        "p" => {
            let mut inlines = Vec::new();
            parse_inlines(node, &mut inlines);
            Some(Block::Paragraph(inlines))
        }
        "cite" => {
            let mut blocks = Vec::new();
            for child in node.children().filter(|n| n.is_element()) {
                if let Some(b) = parse_block_element(child) {
                    blocks.push(b);
                }
            }
            Some(Block::Quote(blocks))
        }
        "epigraph" => {
            let mut blocks = Vec::new();
            for child in node.children().filter(|n| n.is_element()) {
                if let Some(b) = parse_block_element(child) {
                    blocks.push(b);
                }
            }
            Some(Block::Epigraph(blocks))
        }
        "poem" => {
            let mut stanzas = Vec::new();
            for stanza_node in node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "stanza")
            {
                let mut lines = Vec::new();
                for v_node in stanza_node
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "v")
                {
                    let mut line_inlines = Vec::new();
                    parse_inlines(v_node, &mut line_inlines);
                    lines.push(line_inlines);
                }
                stanzas.push(PoemStanza { lines });
            }
            Some(Block::Poem { stanzas })
        }
        "table" => {
            let mut rows = Vec::new();
            for tr in node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "tr")
            {
                let mut cells = Vec::new();
                for td in tr.children().filter(|n| n.is_element()) {
                    let is_header = td.tag_name().name() == "th";
                    let mut inlines = Vec::new();
                    parse_inlines(td, &mut inlines);
                    cells.push(TableCell { is_header, inlines });
                }
                rows.push(TableRow { cells });
            }
            Some(Block::Table { rows })
        }
        "image" => {
            if let Some(href) = node
                .attribute(("http://www.w3.org/1999/xlink", "href"))
                .or_else(|| node.attribute("href"))
            {
                let key = href.trim_start_matches('#').to_string();
                Some(Block::Image { key, alt: None })
            } else {
                None
            }
        }
        "empty-line" => Some(Block::Empty),
        _ => {
            let mut inlines = Vec::new();
            parse_inlines(node, &mut inlines);
            if !inlines.is_empty() {
                Some(Block::Paragraph(inlines))
            } else {
                None
            }
        }
    }
}

fn parse_inlines(node: Node, out: &mut Vec<Inline>) {
    for child in node.children() {
        if child.is_text() {
            if let Some(t) = child.text() {
                if !t.is_empty() {
                    out.push(Inline::Text(t.to_string()));
                }
            }
        } else if child.is_element() {
            match child.tag_name().name() {
                "strong" | "b" => {
                    let mut inner = Vec::new();
                    parse_inlines(child, &mut inner);
                    out.push(Inline::Bold(inner));
                }
                "emphasis" | "i" => {
                    let mut inner = Vec::new();
                    parse_inlines(child, &mut inner);
                    out.push(Inline::Italic(inner));
                }
                "strikethrough" | "s" | "del" => {
                    let mut inner = Vec::new();
                    parse_inlines(child, &mut inner);
                    out.push(Inline::Strike(inner));
                }
                "style" | "code" => {
                    let mut inner = Vec::new();
                    parse_inlines(child, &mut inner);
                    let txt = inner
                        .iter()
                        .map(|i| i.plain_text())
                        .collect::<Vec<_>>()
                        .join("");
                    out.push(Inline::Code(txt));
                }
                "a" => {
                    let target = child
                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                        .or_else(|| child.attribute("href"))
                        .unwrap_or_default()
                        .to_string();
                    let mut inner = Vec::new();
                    parse_inlines(child, &mut inner);
                    out.push(Inline::Link {
                        target,
                        inlines: inner,
                    });
                }
                "image" => {
                    if let Some(href) = child
                        .attribute(("http://www.w3.org/1999/xlink", "href"))
                        .or_else(|| child.attribute("href"))
                    {
                        let key = href.trim_start_matches('#').to_string();
                        out.push(Inline::Image { key, alt: None });
                    }
                }
                _ => {
                    parse_inlines(child, out);
                }
            }
        }
    }
}

fn base64_decode(input: &str) -> std::result::Result<Vec<u8>, ()> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(input))
        .map_err(|_| ())
}
