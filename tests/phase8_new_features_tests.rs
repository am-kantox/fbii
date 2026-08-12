use fbii::config::{Config, KeyAction};
use fbii::db::LibraryDb;
use fbii::formats::model::{Block, Book, Inline, Metadata};
use fbii::formats::{parse_book_file, parse_book_uri};
use fbii::renderer::BookLayout;
use fbii::tui::views::library_view::LibrarySortMode;
use fbii::tui::{App, AppMode};
use tempfile::tempdir;

fn sample_fb2(title: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info>
      <book-title>{}</book-title>
    </title-info>
  </description>
  <body><section><p>{}</p></section></body>
</FictionBook>"#,
        title, body
    )
}

#[test]
fn test_book_identity_is_stable_and_path_derived() {
    // Two different files that happen to share a filename in different
    // directories must not collide (the old filename-only EPUB id and the
    // old content-hash FB2 id both had failure modes here).
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    let path_a = dir_a.path().join("book.fb2");
    let path_b = dir_b.path().join("book.fb2");

    std::fs::write(&path_a, sample_fb2("Book A", "Content A")).unwrap();
    std::fs::write(&path_b, sample_fb2("Book B", "Content B")).unwrap();

    let book_a = parse_book_file(&path_a).unwrap();
    let book_b = parse_book_file(&path_b).unwrap();
    assert_ne!(book_a.id, book_b.id);

    // Re-parsing the same path (even after content changes, e.g. re-saved
    // externally) must yield the same id, so progress/bookmarks survive.
    std::fs::write(&path_a, sample_fb2("Book A", "Edited content")).unwrap();
    let book_a_reparsed = parse_book_file(&path_a).unwrap();
    assert_eq!(book_a.id, book_a_reparsed.id);
}

#[test]
fn test_percent_decoded_non_ascii_path_is_not_mangled() {
    let dir = tempdir().unwrap();
    // Cyrillic filename, to make sure the percent-decoder treats escaped
    // multi-byte UTF-8 sequences as whole bytes rather than casting each
    // escaped byte individually into a `char`.
    let file_name = "книга.fb2";
    let path = dir.path().join(file_name);
    std::fs::write(&path, sample_fb2("Русская книга", "Текст")).unwrap();

    let percent_encoded_name: String = file_name
        .as_bytes()
        .iter()
        .map(|b| format!("%{:02X}", b))
        .collect();
    let uri = format!("{}/{}", dir.path().to_string_lossy(), percent_encoded_name);

    let book = parse_book_uri(&uri).unwrap();
    assert_eq!(book.metadata.title, "Русская книга");
}

#[test]
fn test_progress_percent_matches_status_bar_formula() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = (0..20)
        .map(|i| Block::Paragraph(vec![Inline::Text(format!("Line {}", i))]))
        .collect();

    let config = fbii::config::TypographyConfig::default();
    let layout = BookLayout::build(&book, &config, false);
    let total = layout.lines.len();

    assert_eq!(layout.progress_percent(0, 0), 0.0);
    let expected_full = 100.0;
    assert_eq!(layout.progress_percent(total, 10), expected_full);

    // Matches ((offset + viewport) / total) * 100, clamped to total.
    let offset = 2;
    let viewport = 3;
    let expected = ((offset + viewport).min(total) as f64 / total as f64) * 100.0;
    assert_eq!(layout.progress_percent(offset, viewport), expected);
}

#[test]
fn test_line_spacing_inserts_blank_lines_between_wrapped_lines() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "one two three four five six seven eight nine ten eleven twelve".to_string(),
    )])];

    let narrow_config = fbii::config::TypographyConfig {
        measure: 10,
        line_spacing: 1,
        ..Default::default()
    };
    let single_spaced = BookLayout::build(&book, &narrow_config, false);
    assert!(single_spaced.lines.len() > 1, "expected wrapping to occur");

    let double_spaced_config = fbii::config::TypographyConfig {
        measure: 10,
        line_spacing: 2,
        ..Default::default()
    };
    let double_spaced = BookLayout::build(&book, &double_spaced_config, false);

    // Double spacing should roughly double the number of lines within the
    // wrapped paragraph (one blank line inserted after each wrapped line
    // except the last).
    assert!(double_spaced.lines.len() > single_spaced.lines.len());
}

#[tokio::test]
async fn test_library_sort_filter_and_delete_actions() {
    let db = LibraryDb::new_in_memory().await.unwrap();
    let config = Config::default();
    let config_path = tempdir().unwrap().path().join("config.toml");
    let mut app = App::new(config, db, config_path);

    let book_zebra = Book::new(
        "z1",
        "/books/zebra.fb2",
        Metadata {
            title: "Zebra Tales".to_string(),
            authors: vec!["Alice".to_string()],
            ..Default::default()
        },
    );
    let book_apple = Book::new(
        "a1",
        "/books/apple.fb2",
        Metadata {
            title: "Apple Stories".to_string(),
            authors: vec!["Bob".to_string()],
            ..Default::default()
        },
    );

    app.db.upsert_book(&book_zebra, 0, 0.0).await.unwrap();
    app.db.upsert_book(&book_apple, 0, 0.0).await.unwrap();
    app.refresh_library().await.unwrap();
    assert_eq!(app.library_books.len(), 2);

    // Default sort is recency (DB order); cycling should switch to Title.
    app.mode = AppMode::Library;
    app.handle_action(KeyAction::CycleSort).await;
    assert_eq!(app.library_view.sort_mode, LibrarySortMode::Title);
    let visible = app.visible_library_books();
    assert_eq!(visible[0].title, "Apple Stories");
    assert_eq!(visible[1].title, "Zebra Tales");

    // Filtering narrows the visible set.
    app.library_view.filter = "zebra".to_string();
    let filtered = app.visible_library_books();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].title, "Zebra Tales");
    app.library_view.filter.clear();

    // Deleting the selected (filtered/sorted) book removes it from the DB.
    app.library_view.state.select(Some(0)); // "Apple Stories" under Title sort
    app.handle_action(KeyAction::Delete).await;
    assert_eq!(app.library_books.len(), 1);
    assert_eq!(app.library_books[0].title, "Zebra Tales");
}

#[tokio::test]
async fn test_opds_catalog_persists_to_config_file() {
    let db = LibraryDb::new_in_memory().await.unwrap();
    let config = Config::default();
    let config_dir = tempdir().unwrap();
    let config_path = config_dir.path().join("config.toml");
    let mut app = App::new(config, db, config_path.clone());

    app.opds_catalogs.insert(
        "mycatalog".to_string(),
        "https://example.com/feed.opds".to_string(),
    );
    app.config.opds_catalogs.insert(
        "mycatalog".to_string(),
        "https://example.com/feed.opds".to_string(),
    );
    app.save_config().unwrap();

    let reloaded = Config::load_from_file(&config_path).unwrap();
    assert_eq!(
        reloaded.opds_catalogs.get("mycatalog"),
        Some(&"https://example.com/feed.opds".to_string())
    );
    // Default catalog must still be present after a round trip.
    assert!(reloaded.opds_catalogs.contains_key("gutenberg"));
}

#[tokio::test]
async fn test_toggle_simple_mode_preserves_reading_position() {
    let db = LibraryDb::new_in_memory().await.unwrap();
    let config = Config::default();
    let config_path = tempdir().unwrap().path().join("config.toml");
    let mut app = App::new(config, db, config_path);

    let mut book = Book::new("b1", "/books/test.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text("First paragraph.".to_string())]),
        Block::Quote(vec![Block::Paragraph(vec![Inline::Text(
            "A quoted passage that anchors our reading position.".to_string(),
        )])]),
        Block::Paragraph(vec![Inline::Text("Third paragraph.".to_string())]),
    ];
    app.load_book(book).await;

    // Move into the quoted block, which renders differently (extra quote
    // marker spans) once simplified mode strips the quote formatting.
    // Anchor a few characters past the line start, strictly inside the
    // block's range, to avoid the (pre-existing, boundary-inclusive)
    // ambiguity where an offset exactly on a shared line boundary resolves
    // to the earlier of the two adjacent lines.
    let anchor_char_offset = app
        .active_layout
        .as_ref()
        .unwrap()
        .lines
        .iter()
        .find(|l| l.block_index == 1)
        .unwrap()
        .char_start
        + 5;
    app.reader_view.scroll_offset = app
        .active_layout
        .as_ref()
        .unwrap()
        .line_at_char_offset(anchor_char_offset);

    app.handle_action(KeyAction::ToggleSimpleMode).await;

    let new_line = &app.active_layout.as_ref().unwrap().lines[app.reader_view.scroll_offset];
    // The reading position should still land within (or at the start of)
    // the same source block after the layout rebuild, not at an arbitrary
    // line index left over from the old layout.
    assert_eq!(new_line.block_index, 1);
}
