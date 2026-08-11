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
