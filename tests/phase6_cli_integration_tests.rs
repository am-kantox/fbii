use clap::Parser;
use tabook::cli::CliArgs;
use tabook::config::Config;
use tabook::db::LibraryDb;
use tabook::formats::parse_book_file;
use tabook::tui::{App, AppMode};
use tempfile::NamedTempFile;

#[test]
fn test_cli_argument_parsing() {
    let args = CliArgs::parse_from(["tabook", "--theme", "monokai", "--library"]);
    assert_eq!(args.theme, Some("monokai".to_string()));
    assert!(args.library);
    assert!(args.file_path.is_none());

    let args_file = CliArgs::parse_from(["tabook", "/path/to/book.epub"]);
    assert_eq!(
        args_file.file_path.unwrap().to_str().unwrap(),
        "/path/to/book.epub"
    );
}

#[tokio::test]
async fn test_full_lifecycle_open_file_and_db_persistence() {
    let temp_file = NamedTempFile::new().unwrap();
    let fb2_path = temp_file.path().with_extension("fb2");

    let fb2_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info>
      <book-title>CLI Integration Test Book</book-title>
      <author><first-name>Test</first-name><last-name>Author</last-name></author>
    </title-info>
  </description>
  <body><section><p>Test body paragraph.</p></section></body>
</FictionBook>"#;

    std::fs::write(&fb2_path, fb2_xml).unwrap();

    let db = LibraryDb::new_in_memory().await.unwrap();
    let config = Config::default();
    let mut app = App::new(config, db);

    let book = parse_book_file(&fb2_path).unwrap();
    app.db.upsert_book(&book, 0, 0.0).await.unwrap();
    app.load_book(book);

    assert_eq!(app.mode, AppMode::Reader);
    assert_eq!(
        app.active_book.as_ref().unwrap().metadata.title,
        "CLI Integration Test Book"
    );

    let books_in_db = app.db.list_books().await.unwrap();
    assert_eq!(books_in_db.len(), 1);
    assert_eq!(books_in_db[0].title, "CLI Integration Test Book");

    let _ = std::fs::remove_file(fb2_path);
}
