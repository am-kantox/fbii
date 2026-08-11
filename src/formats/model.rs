use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookFormat {
    Fb2,
    Fb2Zip,
    Epub,
}

impl std::fmt::Display for BookFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookFormat::Fb2 => write!(f, "fb2"),
            BookFormat::Fb2Zip => write!(f, "fb2.zip"),
            BookFormat::Epub => write!(f, "epub"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub title: String,
    pub authors: Vec<String>,
    pub series_name: Option<String>,
    pub series_index: Option<u32>,
    pub genres: Vec<String>,
    pub annotation: Option<String>,
    pub cover_image_key: Option<String>,
    pub format: BookFormat,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            title: "Unknown Title".to_string(),
            authors: vec!["Unknown Author".to_string()],
            series_name: None,
            series_index: None,
            genres: Vec::new(),
            annotation: None,
            cover_image_key: None,
            format: BookFormat::Fb2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TocItem {
    pub title: String,
    pub target_href: String,
    pub block_index: usize,
    pub children: Vec<TocItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strike(Vec<Inline>),
    Link {
        target: String,
        inlines: Vec<Inline>,
    },
    Code(String),
    Image {
        key: String,
        alt: Option<String>,
    },
    LineBreak,
}

impl Inline {
    pub fn plain_text(&self) -> String {
        match self {
            Inline::Text(t) | Inline::Code(t) => t.clone(),
            Inline::Bold(children)
            | Inline::Italic(children)
            | Inline::Underline(children)
            | Inline::Strike(children)
            | Inline::Link {
                inlines: children, ..
            } => children
                .iter()
                .map(|i| i.plain_text())
                .collect::<Vec<_>>()
                .join(""),
            Inline::Image { alt, .. } => alt.clone().unwrap_or_default(),
            Inline::LineBreak => "\n".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListItem {
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    pub is_header: bool,
    pub inlines: Vec<Inline>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoemStanza {
    pub lines: Vec<Vec<Inline>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading { level: u8, inlines: Vec<Inline> },
    Quote(Vec<Block>),
    Epigraph(Vec<Block>),
    Annotation(Vec<Block>),
    List { ordered: bool, items: Vec<ListItem> },
    Table { rows: Vec<TableRow> },
    Poem { stanzas: Vec<PoemStanza> },
    Image { key: String, alt: Option<String> },
    Empty,
}

impl Block {
    pub fn plain_text(&self) -> String {
        match self {
            Block::Paragraph(inlines) | Block::Heading { inlines, .. } => inlines
                .iter()
                .map(|i| i.plain_text())
                .collect::<Vec<_>>()
                .join(""),
            Block::Quote(blocks) | Block::Epigraph(blocks) | Block::Annotation(blocks) => blocks
                .iter()
                .map(|b| b.plain_text())
                .collect::<Vec<_>>()
                .join("\n"),
            Block::List { items, .. } => items
                .iter()
                .map(|item| {
                    item.inlines
                        .iter()
                        .map(|i| i.plain_text())
                        .collect::<Vec<_>>()
                        .join("")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Table { rows } => rows
                .iter()
                .map(|row| {
                    row.cells
                        .iter()
                        .map(|cell| {
                            cell.inlines
                                .iter()
                                .map(|i| i.plain_text())
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Poem { stanzas } => stanzas
                .iter()
                .map(|stanza| {
                    stanza
                        .lines
                        .iter()
                        .map(|line| {
                            line.iter()
                                .map(|i| i.plain_text())
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
            Block::Image { alt, .. } => alt.clone().unwrap_or_default(),
            Block::Empty => String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub id: String,
    pub file_path: String,
    pub metadata: Metadata,
    pub content: Vec<Block>,
    pub toc: Vec<TocItem>,
    pub resources: HashMap<String, Vec<u8>>,
}

impl Book {
    pub fn new(id: impl Into<String>, file_path: impl Into<String>, metadata: Metadata) -> Self {
        Self {
            id: id.into(),
            file_path: file_path.into(),
            metadata,
            content: Vec::new(),
            toc: Vec::new(),
            resources: HashMap::new(),
        }
    }
}
