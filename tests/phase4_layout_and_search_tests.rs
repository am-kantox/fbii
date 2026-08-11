use tabook::config::TypographyConfig;
use tabook::formats::model::{Block, Book, Inline, Metadata};
use tabook::renderer::{simplify_blocks, BookLayout};
use tabook::search::{fold_str, BookSearchIndex};

#[test]
fn test_layout_word_wrapping_and_measure() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "The quick brown fox jumps over the lazy dog near the river bank.".to_string(),
    )])];

    let config = TypographyConfig {
        measure: 30, // Max line width 30 columns
        ..Default::default()
    };

    let layout = BookLayout::build(&book, &config, false);
    assert!(layout.lines.len() > 1);

    for line in &layout.lines {
        let line_text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
        assert!(unicode_width::UnicodeWidthStr::width(line_text.as_str()) <= 32);
    }
}

#[test]
fn test_layout_cjk_wide_character_wrapping() {
    let mut book = Book::new("cjk", "/path/cjk.fb2", Metadata::default());
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "日本語の文章と漢字の折り返しテストです。".to_string(),
    )])];

    let config = TypographyConfig {
        measure: 10, // 10 columns (approx 5 CJK characters)
        ..Default::default()
    };

    let layout = BookLayout::build(&book, &config, false);
    assert!(layout.lines.len() > 1);
}

#[test]
fn test_layout_char_offset_line_mapping() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text("First paragraph content.".to_string())]),
        Block::Paragraph(vec![Inline::Text("Second paragraph content.".to_string())]),
    ];

    let config = TypographyConfig::default();
    let layout = BookLayout::build(&book, &config, false);

    let line_idx = layout.line_at_char_offset(0);
    assert_eq!(line_idx, 0);
}

#[test]
fn test_simplified_mode() {
    let blocks = vec![Block::Quote(vec![Block::Paragraph(vec![Inline::Bold(
        vec![Inline::Text("Quote content".to_string())],
    )])])];

    let simplified = simplify_blocks(&blocks);
    assert_eq!(simplified.len(), 1);
    match &simplified[0] {
        Block::Paragraph(inlines) => {
            assert_eq!(
                inlines[0],
                Inline::Bold(vec![Inline::Text("Quote content".to_string())])
            );
        }
        _ => panic!("Expected paragraph"),
    }
}

#[test]
fn test_nfkd_fold_search() {
    assert_eq!(fold_str("İstanbul"), "istanbul");
    assert_eq!(fold_str("CAFÉ"), "cafe");
    assert_eq!(fold_str("Résumé"), "resume");
    assert_eq!(fold_str("Müller"), "muller");

    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text("Welcome to İstanbul!".to_string())]),
        Block::Paragraph(vec![Inline::Text("Enjoy a hot café au lait.".to_string())]),
    ];

    let index = BookSearchIndex::build(&book);
    let results = index.search("istanbul");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].block_index, 0);

    let cafe_results = index.search("cafe");
    assert_eq!(cafe_results.len(), 1);
    assert_eq!(cafe_results[0].block_index, 1);
}
