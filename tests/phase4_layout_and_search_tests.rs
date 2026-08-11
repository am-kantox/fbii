use tabook::config::TypographyConfig;
use tabook::formats::model::{Block, Book, Inline, Metadata};
use tabook::renderer::{simplify_blocks, BookLayout};
use tabook::search::{fold_str, BookSearchIndex};

#[test]
fn test_layout_word_wrapping_and_measure() {
    let mut book = Book::new("b1", "/path/book.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text(
            "The quick brown fox jumps over the lazy dog near the river bank.".to_string(),
        )]),
    ];

    let mut config = TypographyConfig::default();
    config.measure = 30; // Max line width 30 columns

    let layout = BookLayout::build(&book, &config, false);
    assert!(layout.lines.len() > 1);

    for line in &layout.lines {
        let line_text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
        assert!(unicode_width::UnicodeWidthStr::width(line_text.as_str()) <= 32); // including indent
    }
}

#[test]
fn test_simplified_mode() {
    let blocks = vec![
        Block::Quote(vec![Block::Paragraph(vec![Inline::Bold(vec![
            Inline::Text("Quote content".to_string()),
        ])])]),
    ];

    let simplified = simplify_blocks(&blocks);
    assert_eq!(simplified.len(), 1);
    match &simplified[0] {
        Block::Paragraph(inlines) => {
            assert_eq!(inlines[0], Inline::Bold(vec![Inline::Text("Quote content".to_string())]));
        }
        _ => panic!("Expected paragraph"),
    }
}

#[test]
fn test_nfkd_fold_search() {
    assert_eq!(fold_str("İstanbul"), "istanbul");
    assert_eq!(fold_str("CAFÉ"), "cafe");
    assert_eq!(fold_str("Résumé"), "resume");

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
