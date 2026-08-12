use chrono::Utc;
use fbii::config::Config;
use fbii::db::DbBook;
use fbii::formats::model::{Block, Book, Inline, Metadata};
use fbii::renderer::BookLayout;
use fbii::themes::Theme;
use fbii::tui::views::library_view::LibraryView;
use fbii::tui::views::reader_view::ReaderView;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Render into a fixed-size in-memory terminal and return the full rendered
/// buffer as a single string (row-major, no separators), so tests can
/// assert on substrings without depending on exact cell positions.
fn render_to_text(width: u16, height: u16, draw: impl FnOnce(&mut ratatui::Frame)) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn sample_db_book(id: &str, title: &str, authors: &str) -> DbBook {
    let now = Utc::now();
    DbBook {
        id: id.to_string(),
        file_path: format!("/books/{}.fb2", id),
        title: title.to_string(),
        authors: authors.to_string(),
        series_name: None,
        series_index: None,
        genres: String::new(),
        annotation: None,
        cover_image_key: None,
        format: "fb2".to_string(),
        progress_offset: 0,
        progress_percent: 0.0,
        added_at: now,
        updated_at: now,
    }
}

#[test]
fn test_library_view_render_shows_filtered_books_only() {
    let theme = Theme::get_by_name("nord-dark");
    let books = vec![
        sample_db_book("z1", "Zebra Tales", "Alice"),
        sample_db_book("a1", "Apple Stories", "Bob"),
    ];

    let mut view = LibraryView::new();
    view.filter = "zebra".to_string();

    let text = render_to_text(80, 20, |f| {
        view.render(f, f.area(), &books, &theme, None);
    });

    assert!(text.contains("Zebra Tales"), "rendered buffer: {}", text);
    assert!(!text.contains("Apple Stories"), "rendered buffer: {}", text);
}

#[test]
fn test_library_view_render_shows_status_message() {
    let theme = Theme::get_by_name("nord-dark");
    let books = vec![sample_db_book("a1", "Apple Stories", "Bob")];
    let mut view = LibraryView::new();

    let text = render_to_text(80, 20, |f| {
        view.render(f, f.area(), &books, &theme, Some("Deleted 'Old Book'"));
    });

    assert!(text.contains("Deleted"), "rendered buffer: {}", text);
    assert!(text.contains("Old Book"), "rendered buffer: {}", text);
}

#[test]
fn test_library_view_render_empty_state() {
    let theme = Theme::get_by_name("nord-dark");
    let books: Vec<DbBook> = Vec::new();
    let mut view = LibraryView::new();

    let text = render_to_text(80, 20, |f| {
        view.render(f, f.area(), &books, &theme, None);
    });

    assert!(
        text.contains("Library is empty"),
        "rendered buffer: {}",
        text
    );
}

#[test]
fn test_reader_view_render_shows_title_author_and_help_hint() {
    let theme = Theme::get_by_name("nord-dark");
    let config = Config::default();

    let mut book = Book::new("b1", "/books/test.fb2", Metadata::default());
    book.metadata.title = "The Great Adventure".to_string();
    book.metadata.authors = vec!["A. Writer".to_string()];
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "It was a bright cold day in the story.".to_string(),
    )])];

    let layout = BookLayout::build(&book, &config.typography, false);
    let mut view = ReaderView::new();

    let text = render_to_text(80, 20, |f| {
        view.render(
            f,
            f.area(),
            &book,
            &layout,
            &config,
            &theme,
            None,
            None,
            &[],
            0,
        );
    });

    assert!(
        text.contains("The Great Adventure"),
        "rendered buffer: {}",
        text
    );
    assert!(text.contains("A. Writer"), "rendered buffer: {}", text);
    assert!(text.contains("Help"), "rendered buffer: {}", text);
    assert!(
        text.contains("bright cold day"),
        "rendered buffer: {}",
        text
    );
}

#[test]
fn test_reader_view_render_shows_status_message_instead_of_default_bar() {
    let theme = Theme::get_by_name("nord-dark");
    let config = Config::default();
    let mut book = Book::new("b1", "/books/test.fb2", Metadata::default());
    book.content = vec![Block::Paragraph(vec![Inline::Text(
        "Some content.".to_string(),
    )])];
    let layout = BookLayout::build(&book, &config.typography, false);
    let mut view = ReaderView::new();

    let text = render_to_text(80, 20, |f| {
        view.render(
            f,
            f.area(),
            &book,
            &layout,
            &config,
            &theme,
            Some("No image on the current line."),
            None,
            &[],
            0,
        );
    });

    assert!(
        text.contains("No image on the current line"),
        "rendered buffer: {}",
        text
    );
}
