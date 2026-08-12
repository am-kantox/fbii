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
