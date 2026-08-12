pub mod encoding;
pub mod epub;
pub mod fb2;
pub mod model;

pub use epub::parse_epub;
pub use fb2::{parse_fb2_bytes, parse_fb2_zip};
pub use model::{
    Block, Book, BookFormat, Inline, ListItem, Metadata, PoemStanza, TableCell, TableRow, TocItem,
};

use crate::utils::{AppError, Result};
use std::path::Path;

use sha1::Digest;

pub fn parse_book_uri(uri_str: &str) -> Result<Book> {
    let uri_str = uri_str.trim();

    if uri_str.starts_with("http://") || uri_str.starts_with("https://") {
        let response = reqwest::blocking::get(uri_str)
            .map_err(|e| AppError::Parse(format!("Failed to download URI '{}': {}", uri_str, e)))?;

        if !response.status().is_success() {
            return Err(AppError::Parse(format!(
                "HTTP error {} downloading URI '{}'",
                response.status(),
                uri_str
            )));
        }

        let bytes = response
            .bytes()
            .map_err(|e| AppError::Parse(format!("Failed to read HTTP response bytes: {}", e)))?;

        let ext = if uri_str.to_lowercase().contains(".fb2.zip") {
            ".fb2.zip"
        } else if uri_str.to_lowercase().contains(".epub") {
            ".epub"
        } else if uri_str.to_lowercase().contains(".fb2") {
            ".fb2"
        } else {
            ".tmp"
        };

        let temp_dir = std::env::temp_dir();
        let hash = format!("{:x}", sha1::Sha1::digest(&bytes));
        let temp_path = temp_dir.join(format!("fbii_remote_{}{}", hash, ext));
        std::fs::write(&temp_path, &bytes)?;

        let book = parse_book_file(&temp_path)?;
        let _ = std::fs::remove_file(temp_path);
        Ok(book)
    } else {
        let path_str = if let Some(stripped) = uri_str.strip_prefix("file://") {
            if let Some(rest) = stripped.strip_prefix("localhost") {
                rest.to_string()
            } else {
                stripped.to_string()
            }
        } else if let Some(stripped) = uri_str.strip_prefix("file:") {
            stripped.to_string()
        } else {
            uri_str.to_string()
        };

        let decoded_path = percent_encoding_decode(&path_str);
        let path = if let Some(stripped) = decoded_path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(stripped)
            } else {
                std::path::PathBuf::from(&decoded_path)
            }
        } else {
            std::path::PathBuf::from(&decoded_path)
        };

        parse_book_file(&path)
    }
}

fn percent_encoding_decode(s: &str) -> String {
    let mut result = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(hex_val as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

pub fn parse_book_file(path: &Path) -> Result<Book> {
    let path_str = path.to_string_lossy().to_lowercase();

    if path_str.ends_with(".fb2.zip") {
        parse_fb2_zip(path)
    } else if path_str.ends_with(".fb2") {
        let bytes = std::fs::read(path)?;
        parse_fb2_bytes(&bytes, &path.to_string_lossy())
    } else if path_str.ends_with(".epub") {
        parse_epub(path)
    } else {
        // Try best effort: check file header / extension
        if let Ok(bytes) = std::fs::read(path) {
            if bytes.starts_with(b"PK\x03\x04") {
                // ZIP file: try EPUB first, then FB2.ZIP
                if let Ok(book) = parse_epub(path) {
                    return Ok(book);
                }
                if let Ok(book) = parse_fb2_zip(path) {
                    return Ok(book);
                }
            } else {
                // Try raw FB2 XML
                if let Ok(book) = parse_fb2_bytes(&bytes, &path.to_string_lossy()) {
                    return Ok(book);
                }
            }
        }
        Err(AppError::Parse(format!(
            "Unsupported or unrecognized e-book format for file: {}",
            path.display()
        )))
    }
}
