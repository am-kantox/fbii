use fbii::config::{KeyAction, KeyMap, TypographyConfig};
use fbii::db::LibraryDb;
use fbii::formats::epub::parse_epub;
use fbii::formats::model::{Block, Book, Inline, Metadata};
use fbii::formats::{parse_book_uri, parse_book_uri_async};
use fbii::renderer::BookLayout;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_readme_keybindings_mapping() {
    let keymap = KeyMap::default();
    let bindings = &keymap.bindings;

    // Verify keybindings listed in README.md exist in default keymap
    assert_eq!(bindings.get("j"), Some(&KeyAction::ScrollDown));
    assert_eq!(bindings.get("Down"), Some(&KeyAction::ScrollDown));
    assert_eq!(bindings.get("k"), Some(&KeyAction::ScrollUp));
    assert_eq!(bindings.get("Up"), Some(&KeyAction::ScrollUp));
    assert_eq!(bindings.get("ctrl+f"), Some(&KeyAction::PageDown));
    assert_eq!(bindings.get("ctrl+b"), Some(&KeyAction::PageUp));
    assert_eq!(bindings.get("ctrl+d"), Some(&KeyAction::HalfPageDown));
    assert_eq!(bindings.get("ctrl+u"), Some(&KeyAction::HalfPageUp));
    assert_eq!(bindings.get("gg"), Some(&KeyAction::GotoTop));
    assert_eq!(bindings.get("G"), Some(&KeyAction::GotoBottom));
    assert_eq!(bindings.get("/"), Some(&KeyAction::Search));
    assert_eq!(bindings.get("n"), Some(&KeyAction::NextMatch));
    assert_eq!(bindings.get("N"), Some(&KeyAction::PrevMatch));
    assert_eq!(bindings.get("t"), Some(&KeyAction::Toc));
    assert_eq!(bindings.get("b"), Some(&KeyAction::AddBookmark));
    assert_eq!(bindings.get("B"), Some(&KeyAction::ListBookmarks));
    assert_eq!(bindings.get("d"), Some(&KeyAction::Delete));
    assert_eq!(bindings.get("r"), Some(&KeyAction::CycleSort));
    assert_eq!(bindings.get("v"), Some(&KeyAction::ViewImage));
    assert_eq!(bindings.get("i"), Some(&KeyAction::Info));
    assert_eq!(bindings.get("S"), Some(&KeyAction::ToggleSimpleMode));
    assert_eq!(bindings.get("C"), Some(&KeyAction::ToggleCss));
    assert_eq!(bindings.get("?"), Some(&KeyAction::Help));
    assert_eq!(bindings.get("q"), Some(&KeyAction::Quit));
    assert_eq!(bindings.get(":"), Some(&KeyAction::Command));
}

#[test]
fn test_typography_layout_options() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text(
            "First long word supercalifragilisticexpialidocious sentence.".to_string(),
        )]),
        Block::Paragraph(vec![Inline::Text(
            "Second paragraph containing multiple words to test justification.".to_string(),
        )]),
    ];

    // 1. Test paragraph_indent
    let indent_config = TypographyConfig {
        paragraph_indent: 4,
        paragraph_spacing: 0,
        line_spacing: 0,
        measure: 80,
        hyphenation: false,
        justified: false,
    };
    let layout_indent = BookLayout::build(&book, &indent_config, false);
    let first_line_text: String = layout_indent.lines[0]
        .spans
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert!(
        first_line_text.starts_with("    First"),
        "First line should start with 4 indent spaces: '{}'",
        first_line_text
    );

    // 2. Test paragraph_spacing
    let spacing_config = TypographyConfig {
        paragraph_indent: 0,
        paragraph_spacing: 2,
        line_spacing: 0,
        measure: 80,
        hyphenation: false,
        justified: false,
    };
    let layout_spacing = BookLayout::build(&book, &spacing_config, false);
    let empty_lines = layout_spacing
        .lines
        .iter()
        .filter(|l| l.is_empty_line)
        .count();
    assert_eq!(
        empty_lines, 2,
        "Should insert 2 empty lines for paragraph_spacing"
    );

    // 3. Test hyphenation
    let hyphen_config = TypographyConfig {
        paragraph_indent: 0,
        paragraph_spacing: 0,
        line_spacing: 0,
        measure: 15, // Narrow measure so long word breaks
        hyphenation: true,
        justified: false,
    };
    let layout_hyphen = BookLayout::build(&book, &hyphen_config, false);
    let has_hyphen = layout_hyphen.lines.iter().any(|l| {
        l.spans
            .iter()
            .any(|s| s.text.ends_with('-') || s.text.contains('-'))
    });
    assert!(
        has_hyphen,
        "Hyphenation should split long words exceeding measure"
    );

    // 4. Test justified text
    let justify_config = TypographyConfig {
        paragraph_indent: 0,
        paragraph_spacing: 0,
        line_spacing: 0,
        measure: 30,
        hyphenation: false,
        justified: true,
    };
    let layout_justify = BookLayout::build(&book, &justify_config, false);
    assert!(!layout_justify.lines.is_empty());
}

#[test]
fn test_epub_css_display_none_parsing_and_toggle() {
    let dir = tempdir().unwrap();
    let epub_path = dir.path().join("hidden_test.epub");

    // Construct a minimal EPUB with inline style="display: none"
    let file = File::create(&epub_path).unwrap();
    let mut zip = ZipWriter::new(file);

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("mimetype", options).unwrap();
    zip.write_all(b"application/epub+zip").unwrap();

    zip.start_file("META-INF/container.xml", options).unwrap();
    zip.write_all(
        b"<?xml version=\"1.0\"?>
        <container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">
          <rootfiles>
            <rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>
          </rootfiles>
        </container>",
    )
    .unwrap();

    zip.start_file("OEBPS/content.opf", options).unwrap();
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"utf-8\"?>
        <package version=\"3.0\" unique-identifier=\"BookId\" xmlns=\"http://www.idpf.org/2007/opf\">
          <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">
            <dc:title>CSS Hidden Test</dc:title>
          </metadata>
          <manifest>
            <item id=\"ch1\" href=\"ch1.xhtml\" media-type=\"application/xhtml+xml\"/>
          </manifest>
          <spine>
            <itemref idref=\"ch1\"/>
          </spine>
        </package>",
    )
    .unwrap();

    zip.start_file("OEBPS/ch1.xhtml", options).unwrap();
    zip.write_all(
        b"<?xml version=\"1.0\" encoding=\"utf-8\"?>
        <html xmlns=\"http://www.w3.org/1999/xhtml\">
          <body>
            <p>Visible content before.</p>
            <p style=\"display: none;\">Secret hidden content.</p>
            <p>Visible content after.</p>
          </body>
        </html>",
    )
    .unwrap();

    zip.finish().unwrap();

    let book = parse_epub(&epub_path).unwrap();
    let has_hidden_block = book.content.iter().any(|b| matches!(b, Block::Hidden(_)));
    assert!(
        has_hidden_block,
        "EPUB parser should convert display:none elements to Block::Hidden"
    );

    // Test layout with respect_css = true (hidden by default)
    let config = TypographyConfig::default();
    let layout_hidden = BookLayout::build_with_css(&book, &config, false, true);
    let full_text_hidden: String = layout_hidden
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.text.as_str())
        .collect();
    assert!(!full_text_hidden.contains("Secret hidden content"));
    assert!(full_text_hidden.contains("Visible content before"));

    // Test layout with respect_css = false (reveal hidden content)
    let layout_revealed = BookLayout::build_with_css(&book, &config, false, false);
    let full_text_revealed: String = layout_revealed
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.text.as_str())
        .collect();
    assert!(full_text_revealed.contains("Secret hidden content"));
}

#[tokio::test]
async fn test_db_reading_stats_aggregation() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let book = Book::new("b_stats", "/path/stats.epub", Metadata::default());
    db.upsert_book(&book, 0, 0.0).await.unwrap();

    let s1 = db.start_reading_session("b_stats").await.unwrap();
    db.end_reading_session(&s1, 10).await.unwrap();

    let s2 = db.start_reading_session("b_stats").await.unwrap();
    db.end_reading_session(&s2, 15).await.unwrap();

    let stats = db.get_reading_stats("b_stats").await.unwrap();
    assert_eq!(stats.sessions, 2);
    assert_eq!(stats.total_pages, 25);
}

#[tokio::test]
async fn test_uri_resolution_helpers() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.fb2");

    let fb2_xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body><section><p>Hello URI</p></section></body></FictionBook>";
    std::fs::write(&file_path, fb2_xml).unwrap();

    // Test parse_book_uri with file:// URI
    let file_uri = format!("file://{}", file_path.to_string_lossy());
    let book = parse_book_uri(&file_uri).unwrap();
    assert_eq!(book.metadata.title, "Unknown Title");

    // Test parse_book_uri_async with file:// URI
    let book_async = parse_book_uri_async(&file_uri).await.unwrap();
    assert_eq!(book_async.file_path, file_path.to_string_lossy());

    // Test parse_book_uri_async with http error case
    let http_res = parse_book_uri_async("http://127.0.0.1:1/nonexistent.epub").await;
    assert!(http_res.is_err());
}

#[test]
fn test_i_r_j_w_hotkeys_and_search_highlighting() {
    let keymap = KeyMap::default();
    let bindings = &keymap.bindings;

    // Verify hotkeys: i (Book info), R (Toggle recent books / CycleSort), J (Toggle text justify), W (Toggle wide screen)
    assert_eq!(bindings.get("i"), Some(&KeyAction::Info));
    assert_eq!(bindings.get("R"), Some(&KeyAction::CycleSort));
    assert_eq!(bindings.get("J"), Some(&KeyAction::ToggleJustify));
    assert_eq!(bindings.get("W"), Some(&KeyAction::ToggleWidescreen));

    // Verify search highlighting logic with SearchMatch
    let mut book = Book::new("b_search", "/path/search.fb2", Metadata::default());
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "The quick brown fox jumps over the lazy dog.".to_string(),
    )])];

    let index = fbii::search::BookSearchIndex::build(&book);
    let matches = index.search("fox");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].char_start, 16);
    assert_eq!(matches[0].char_end, 19);

    let layout = BookLayout::build(&book, &TypographyConfig::default(), false);
    assert!(!layout.lines.is_empty());
}
