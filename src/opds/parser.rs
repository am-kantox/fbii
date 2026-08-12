use crate::opds::model::{OpdsEntry, OpdsFeed, OpdsLinkType};
use crate::utils::{AppError, Result};
use roxmltree::Node;

pub fn parse_opds_feed(xml_str: &str, base_url: &str) -> Result<OpdsFeed> {
    let doc = roxmltree::Document::parse(xml_str)
        .map_err(|e| AppError::Parse(format!("Failed to parse OPDS XML feed: {}", e)))?;

    let root = doc.root_element();
    if !root.tag_name().name().eq_ignore_ascii_case("feed") {
        return Err(AppError::Parse("Root element is not <feed>".to_string()));
    }

    let title = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("title"))
        .map(|n| n.text().unwrap_or("OPDS Catalog").trim().to_string())
        .unwrap_or_else(|| "OPDS Catalog".to_string());

    let mut next_url = None;
    let mut search_url = None;

    for link_node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("link"))
    {
        let rel = link_node.attribute("rel").unwrap_or_default();
        let href = link_node.attribute("href").unwrap_or_default();
        let link_type = link_node.attribute("type").unwrap_or_default();

        if !href.is_empty() {
            let full_href = resolve_url(base_url, href);
            if rel == "next" {
                next_url = Some(full_href);
            } else if rel == "search" || link_type.contains("opensearch") {
                search_url = Some(full_href);
            }
        }
    }

    let mut entries = Vec::new();

    for entry_node in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("entry"))
    {
        if let Some(entry) = parse_opds_entry(entry_node, base_url) {
            entries.push(entry);
        }
    }

    Ok(OpdsFeed {
        title,
        url: base_url.to_string(),
        next_url,
        search_url,
        entries,
    })
}

fn parse_opds_entry(node: Node, base_url: &str) -> Option<OpdsEntry> {
    let title = node
        .children()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("title"))?
        .text()?
        .trim()
        .to_string();

    let author = node
        .children()
        .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("author"))
        .and_then(|a| {
            a.children()
                .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("name"))
                .and_then(|n| n.text())
        })
        .map(|s| s.trim().to_string())
        .or_else(|| {
            node.children()
                .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("content"))
                .and_then(|c| c.text())
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown Author".to_string());

    let summary = node
        .children()
        .find(|n| {
            n.is_element()
                && (n.tag_name().name().eq_ignore_ascii_case("summary")
                    || n.tag_name().name().eq_ignore_ascii_case("content"))
        })
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let mut acq_link = None;
    let mut catalog_link = None;

    for link in node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("link"))
    {
        let href = match link.attribute("href") {
            Some(h) if !h.is_empty() => resolve_url(base_url, h),
            _ => continue,
        };

        let rel = link.attribute("rel").unwrap_or_default();
        let link_type = link.attribute("type").unwrap_or_default();

        if (rel.contains("acquisition")
            || link_type.contains("epub")
            || link_type.contains("fb2")
            || href.ends_with(".epub")
            || href.ends_with(".fb2")
            || href.ends_with(".fb2.zip")
            || href.contains(".epub."))
            && (acq_link.is_none() || link_type.contains("epub"))
        {
            acq_link = Some(href);
        } else if (rel.contains("subsection")
            || rel.contains("related")
            || link_type.contains("opds-catalog")
            || link_type.contains("atom+xml")
            || href.ends_with(".opds"))
            && catalog_link.is_none()
        {
            catalog_link = Some(href);
        }
    }

    let link = if let Some(acq) = acq_link {
        OpdsLinkType::Acquisition(acq)
    } else if let Some(cat) = catalog_link {
        OpdsLinkType::Catalog(cat)
    } else {
        return None;
    };

    Some(OpdsEntry {
        title,
        author,
        summary,
        link,
    })
}

pub fn resolve_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        if let Ok(url) = reqwest::Url::parse(base) {
            format!("{}://{}{}", url.scheme(), url.authority(), href)
        } else {
            href.to_string()
        }
    } else {
        format!("{}/{}", base.trim_end_matches('/'), href)
    }
}
