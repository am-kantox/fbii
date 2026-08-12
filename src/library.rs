//! Library directory scanning.
//!
//! Populating the library previously required opening files one at a time
//! (`:open`, CLI file argument, or OPDS download). This module adds the
//! ability to recursively scan a directory for recognized e-book files and
//! bulk-import them into the database, without clobbering the progress of
//! books that are already tracked.

use crate::db::LibraryDb;
use std::path::{Path, PathBuf};

/// Recursively walk `dir` and collect paths to every recognized e-book file
/// (`.fb2`, `.fb2.zip`, `.epub`), matched case-insensitively. Unreadable
/// subdirectories are silently skipped rather than aborting the whole scan.
pub fn scan_directory(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    scan_directory_into(dir, &mut results);
    results
}

fn scan_directory_into(dir: &Path, results: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_directory_into(&path, results);
        } else if is_supported_book_file(&path) {
            results.push(path);
        }
    }
}

fn is_supported_book_file(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    lower.ends_with(".fb2") || lower.ends_with(".fb2.zip") || lower.ends_with(".epub")
}

/// Outcome of a directory scan/import pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanSummary {
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl ScanSummary {
    pub fn total(&self) -> usize {
        self.imported + self.skipped + self.failed
    }
}

/// Recursively scan `dir` for e-book files and import any that are not
/// already tracked in `db`. Books already present (matched by their stable
/// path-derived id) are left untouched so an existing reading progress is
/// never reset by a re-scan.
pub async fn scan_and_import(db: &LibraryDb, dir: &Path) -> ScanSummary {
    let files = scan_directory(dir);
    let mut summary = ScanSummary::default();

    for path in files {
        match crate::formats::parse_book_file(&path) {
            Ok(book) => {
                let already_known = db.get_book_by_id(&book.id).await.ok().flatten().is_some();
                if already_known {
                    summary.skipped += 1;
                } else if db.upsert_book(&book, 0, 0.0).await.is_ok() {
                    summary.imported += 1;
                } else {
                    summary.failed += 1;
                }
            }
            Err(_) => summary.failed += 1,
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_directory_finds_supported_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let sub_dir = dir.path().join("nested");
        std::fs::create_dir_all(&sub_dir).unwrap();

        std::fs::write(dir.path().join("book1.fb2"), b"data").unwrap();
        std::fs::write(dir.path().join("book2.EPUB"), b"data").unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"data").unwrap();
        std::fs::write(sub_dir.join("book3.fb2.zip"), b"data").unwrap();

        let mut found = scan_directory(dir.path());
        found.sort();

        assert_eq!(found.len(), 3);
        assert!(found
            .iter()
            .any(|p| p.to_string_lossy().ends_with("book1.fb2")));
        assert!(found
            .iter()
            .any(|p| p.to_string_lossy().ends_with("book2.EPUB")));
        assert!(found
            .iter()
            .any(|p| p.to_string_lossy().ends_with("book3.fb2.zip")));
    }

    #[test]
    fn test_scan_directory_missing_dir_returns_empty() {
        let missing = Path::new("/nonexistent/definitely/not/a/real/path");
        assert_eq!(scan_directory(missing), Vec::<PathBuf>::new());
    }

    #[tokio::test]
    async fn test_scan_and_import_skips_already_known_books() {
        let dir = tempfile::tempdir().unwrap();
        let fb2_path = dir.path().join("book.fb2");
        std::fs::write(
            &fb2_path,
            br#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
<description><title-info><book-title>Test</book-title></title-info></description>
<body><section><p>Hello world</p></section></body>
</FictionBook>"#,
        )
        .unwrap();

        let db = LibraryDb::new_in_memory().await.unwrap();

        let summary = scan_and_import(&db, dir.path()).await;
        assert_eq!(summary.imported, 1);
        assert_eq!(summary.skipped, 0);

        // Simulate progress having been made, then re-scan: it must not reset.
        let book = crate::formats::parse_book_file(&fb2_path).unwrap();
        db.upsert_book(&book, 42, 55.5).await.unwrap();

        let summary2 = scan_and_import(&db, dir.path()).await;
        assert_eq!(summary2.imported, 0);
        assert_eq!(summary2.skipped, 1);

        let stored = db.get_book_by_id(&book.id).await.unwrap().unwrap();
        assert_eq!(stored.progress_offset, 42);
    }
}
