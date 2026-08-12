#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpdsLinkType {
    Catalog(String),
    Acquisition(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpdsEntry {
    pub title: String,
    pub author: String,
    pub summary: String,
    pub link: OpdsLinkType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpdsFeed {
    pub title: String,
    pub url: String,
    pub next_url: Option<String>,
    pub search_url: Option<String>,
    pub entries: Vec<OpdsEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opds_link_type_equality_and_clone() {
        let a = OpdsLinkType::Catalog("https://example.com/catalog".to_string());
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(
            OpdsLinkType::Catalog("x".to_string()),
            OpdsLinkType::Acquisition("x".to_string())
        );
    }

    #[test]
    fn test_opds_entry_and_feed_construction() {
        let entry = OpdsEntry {
            title: "Pride and Prejudice".to_string(),
            author: "Jane Austen".to_string(),
            summary: "A classic novel.".to_string(),
            link: OpdsLinkType::Acquisition("https://example.com/book.epub".to_string()),
        };
        let feed = OpdsFeed {
            title: "Test Catalog".to_string(),
            url: "https://example.com/opds".to_string(),
            next_url: Some("https://example.com/opds?page=2".to_string()),
            search_url: None,
            entries: vec![entry.clone()],
        };

        assert_eq!(feed.entries.len(), 1);
        assert_eq!(feed.entries[0], entry);
        assert!(feed.search_url.is_none());
        assert_eq!(
            feed.next_url.as_deref(),
            Some("https://example.com/opds?page=2")
        );

        // Debug formatting should not panic and should mention the title,
        // as a smoke test that the derive is present and working.
        assert!(format!("{:?}", feed).contains("Test Catalog"));
    }
}
