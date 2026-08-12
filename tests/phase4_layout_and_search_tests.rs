use fbii::config::TypographyConfig;
use fbii::formats::model::{Block, Book, Inline, Metadata};
use fbii::renderer::{simplify_blocks, BookLayout};
use fbii::search::{fold_str, BookSearchIndex};

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
    let cjk_text = "日本語の文章と漢字の折り返しテストです。";
    book.content = vec![Block::Paragraph(vec![Inline::Text(cjk_text.to_string())])];

    let config = TypographyConfig {
        measure: 10, // 10 columns (approx 5 CJK characters)
        paragraph_indent: 0,
        ..Default::default()
    };

    let layout = BookLayout::build(&book, &config, false);
    // A 20-character, space-free CJK string wrapped at ~5 fullwidth chars
    // per line should yield several lines, not a single bisection into two
    // (the old, incorrect behavior for text with no ASCII spaces at all).
    assert!(layout.lines.len() > 3);

    for line in &layout.lines {
        let line_text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
        assert!(
            unicode_width::UnicodeWidthStr::width(line_text.as_str()) <= 10,
            "line '{}' exceeds the configured measure",
            line_text
        );
    }

    // No characters should be lost or reordered by the wrapping/tokenizing
    // fix, and no hyphens should be introduced for a CJK break.
    let rejoined: String = layout
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(rejoined, cjk_text);
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

#[test]
fn test_search_matches_map_correctly_to_layout_lines() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text("First block header.".to_string())]),
        Block::Paragraph(vec![Inline::Text(
            "Second block with unique target word inside.".to_string(),
        )]),
        Block::Paragraph(vec![Inline::Text("Third block closing.".to_string())]),
    ];

    let config = TypographyConfig::default();
    let layout = BookLayout::build(&book, &config, false);
    let index = BookSearchIndex::build(&book);

    let matches = index.search("target");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].block_index, 1);

    let line_idx = layout.line_at_char_offset(matches[0].char_start);
    assert!(line_idx > 0 && line_idx < layout.lines.len() - 1);
}
