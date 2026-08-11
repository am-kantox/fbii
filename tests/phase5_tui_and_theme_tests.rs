use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tabook::config::{Config, KeyAction, KeyMap};
use tabook::db::LibraryDb;
use tabook::formats::model::{Block, Book, Inline, Metadata};
use tabook::themes::Theme;
use tabook::tui::keymap_dispatcher::KeymapDispatcher;
use tabook::tui::{App, AppMode};

#[test]
fn test_theme_catalog() {
    let dracula = Theme::get_by_name("dracula");
    assert_eq!(dracula.name, "dracula");

    let monokai = Theme::get_by_name("monokai");
    assert_eq!(monokai.name, "monokai");

    let gh_dark = Theme::get_by_name("github-dark");
    assert_eq!(gh_dark.name, "github-dark");

    let fallback = Theme::get_by_name("unknown-theme");
    assert_eq!(fallback.name, "dracula");
}

#[test]
fn test_keymap_dispatcher() {
    let keymap = KeyMap::default();
    let mut dispatcher = KeymapDispatcher::new();

    // Single key 'j' -> ScrollDown
    let j_event = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let action = dispatcher.handle_event(j_event, &keymap);
    assert_eq!(action, Some(KeyAction::ScrollDown));

    // Multi-key 'g' then 'g' -> GotoTop
    let g_event = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
    let action1 = dispatcher.handle_event(g_event, &keymap);
    assert_eq!(action1, None);

    let action2 = dispatcher.handle_event(g_event, &keymap);
    assert_eq!(action2, Some(KeyAction::GotoTop));
}

#[tokio::test]
async fn test_app_action_handling() {
    let config = Config::default();
    let db = LibraryDb::new_in_memory().await.unwrap();
    let mut app = App::new(config, db);

    assert_eq!(app.mode, AppMode::Library);

    let mut book = Book::new("b1", "/books/test.fb2", Metadata::default());
    book.content = vec![
        Block::Paragraph(vec![Inline::Text("Line 1".to_string())]),
        Block::Paragraph(vec![Inline::Text("Line 2".to_string())]),
        Block::Paragraph(vec![Inline::Text("Line 3".to_string())]),
    ];

    app.load_book(book);
    assert_eq!(app.mode, AppMode::Reader);
    assert_eq!(app.reader_view.scroll_offset, 0);

    // Test scroll actions
    app.handle_action(KeyAction::ScrollDown);
    assert_eq!(app.reader_view.scroll_offset, 1);

    app.handle_action(KeyAction::ScrollUp);
    assert_eq!(app.reader_view.scroll_offset, 0);

    // Test TOC modal toggle
    app.handle_action(KeyAction::Toc);
    assert!(app.reader_view.show_toc);

    // Quit while TOC is open closes TOC
    app.handle_action(KeyAction::Quit);
    assert!(!app.reader_view.show_toc);
    assert_eq!(app.mode, AppMode::Reader);

    // Quit while in Reader switches to Library
    app.handle_action(KeyAction::Quit);
    assert_eq!(app.mode, AppMode::Library);

    // Quit while in Library stops app
    app.handle_action(KeyAction::Quit);
    assert!(!app.is_running);
}
