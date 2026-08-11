use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypographyConfig {
    pub measure: u16,
    pub line_spacing: u8,
    pub paragraph_indent: u8,
    pub paragraph_spacing: u8,
    pub hyphenation: bool,
}

impl Default for TypographyConfig {
    fn default() -> Self {
        Self {
            measure: 80,
            line_spacing: 1,
            paragraph_indent: 2,
            paragraph_spacing: 1,
            hyphenation: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisplayConfig {
    pub simplified_mode: bool,
    pub respect_epub_css: bool,
    pub image_protocol: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            simplified_mode: false,
            respect_epub_css: true,
            image_protocol: "auto".to_string(),
        }
    }
}
