pub mod encoding;
pub mod epub;
pub mod fb2;
pub mod model;

pub use epub::parse_epub;
pub use fb2::{parse_fb2_bytes, parse_fb2_zip};
pub use model::{
    Block, Book, BookFormat, Inline, ListItem, Metadata, PoemStanza, TableCell, TableRow, TocItem,
};

use crate::utils::{sha1_hex, AppError, Result};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Shared async HTTP client with sane connect/request timeouts, reused across
/// OPDS feed fetches and remote book downloads so a slow or dead server can
/// never hang the process indefinitely.
pub(crate) fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default()
    })
}

/// Shared blocking HTTP client, mirroring [`http_client`] for the synchronous
/// `parse_book_uri` entry point.
fn http_blocking_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default()
    })
}

/// Derive a stable book identifier from a file path rather than its content,
/// so re-saving/editing a book externally (e.g. in Calibre) does not silently
/// orphan existing reading progress and bookmarks. Falls back to the given
/// path if it cannot be canonicalized (e.g. it no longer exists on disk).
pub(crate) fn book_id_for_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    sha1_hex(canonical.to_string_lossy().as_bytes())
}

fn is_http_uri(uri_str: &str) -> bool {
    uri_str.starts_with("http://") || uri_str.starts_with("https://")
}

fn sniff_remote_extension(uri_str: &str) -> &'static str {
    let lower = uri_str.to_lowercase();
    if lower.contains(".fb2.zip") {
        ".fb2.zip"
    } else if lower.contains(".epub") {
        ".epub"
    } else if lower.contains(".fb2") {
        ".fb2"
    } else {
        ".tmp"
    }
}

/// Write downloaded bytes to a temp file (named after the remote URI's
/// sniffed extension) and parse it, cleaning up the temp file afterwards.
/// Shared by both the sync and async download paths.
fn parse_book_from_remote_bytes(bytes: &[u8], uri_str: &str) -> Result<Book> {
    let ext = sniff_remote_extension(uri_str);
    let temp_dir = std::env::temp_dir();
    let hash = sha1_hex(bytes);
    let temp_path = temp_dir.join(format!("fbii_remote_{}{}", hash, ext));
    std::fs::write(&temp_path, bytes)?;

    let book = parse_book_file(&temp_path);
    let _ = std::fs::remove_file(&temp_path);
    book
}

/// Resolve a `file://`/bare-path URI string into a local filesystem path,
/// handling `file://`, `file:`, `~/` and percent-encoded paths. Shared by
/// both the sync and async entry points.
fn resolve_local_uri_path(uri_str: &str) -> PathBuf {
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
    if let Some(stripped) = decoded_path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(&decoded_path)
}

pub async fn parse_book_uri_async(uri_str: &str) -> Result<Book> {
    let uri_str = uri_str.trim();

    if is_http_uri(uri_str) {
        let response =
            http_client().get(uri_str).send().await.map_err(|e| {
                AppError::Parse(format!("Failed to download URI '{}': {}", uri_str, e))
            })?;

        if !response.status().is_success() {
            return Err(AppError::Parse(format!(
                "HTTP error {} downloading URI '{}'",
                response.status(),
                uri_str
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Parse(format!("Failed to read HTTP response bytes: {}", e)))?;

        parse_book_from_remote_bytes(&bytes, uri_str)
    } else {
        parse_book_uri(uri_str)
    }
}

pub fn parse_book_uri(uri_str: &str) -> Result<Book> {
    let uri_str = uri_str.trim();

    if is_http_uri(uri_str) {
        let response = http_blocking_client()
            .get(uri_str)
            .send()
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

        parse_book_from_remote_bytes(&bytes, uri_str)
    } else {
        let path = resolve_local_uri_path(uri_str);
        parse_book_file(&path)
    }
}

/// Percent-decode a URI path component. Escaped bytes are accumulated as raw
/// bytes (not cast one-by-one into `char`s) so that multi-byte UTF-8
/// sequences — whether percent-escaped or literal — are decoded correctly
/// instead of being mangled into invalid/incorrect characters.
fn percent_encoding_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out_bytes: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out_bytes.push(hex_val);
                i += 3;
                continue;
            }
        }
        out_bytes.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out_bytes).into_owned()
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
