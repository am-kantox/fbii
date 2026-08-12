pub mod errors;

pub use errors::{AppError, Result};

/// Compute the lowercase hex-encoded SHA1 digest of the given bytes.
///
/// Used throughout the crate to derive stable identifiers (book ids, bookmark
/// ids, session ids, temp file names) from arbitrary byte content.
pub fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::Digest;
    let hash = sha1::Sha1::digest(bytes);
    format!("{:x}", hash)
}
