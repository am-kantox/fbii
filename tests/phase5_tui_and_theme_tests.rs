use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fbii::config::{Config, KeyAction, KeyMap};
use fbii::db::LibraryDb;
use fbii::formats::model::{Block, Book, Inline, Metadata};
use fbii::themes::Theme;
use fbii::tui::keymap_dispatcher::{format_key_event, KeymapDispatcher};
use fbii::tui::{App, AppMode};

#[test]
fn test_theme_catalog() {
    let nord_dark = Theme::get_by_name("nord-dark");
    assert_eq!(nord_dark.name, "nord-dark");

    let nord_light = Theme::get_by_name("nord-light");
    assert_eq!(nord_light.name, "nord-light");

    let system_theme = Theme::get_by_name("auto");
    assert!(system_theme.name == "nord-dark" || system_theme.name == "nord-light");

    let dracula = Theme::get_by_name("dracula");
    assert_eq!(dracula.name, "dracula");
    assert!(dracula.base_style().bg.is_some());

    let monokai = Theme::get_by_name("monokai");
    assert_eq!(monokai.name, "monokai");

    let gh_dark = Theme::get_by_name("github-dark");
    assert_eq!(gh_dark.name, "github-dark");

    let gh_light = Theme::get_by_name("github-light");
    assert_eq!(gh_light.name, "github-light");

    let fallback = Theme::get_by_name("unknown-theme");
    assert!(fallback.name == "nord-dark" || fallback.name == "nord-light");
}

#[test]
fn test_key_event_formatting() {
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(format_key_event(enter), "Enter");

    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(format_key_event(esc), "Esc");

    let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(format_key_event(ctrl_d), "ctrl+d");
}

#[test]
fn test_keymap_dispatcher() {
    let keymap = KeyMap::default();
    let mut dispatcher = KeymapDispatcher::new();

    // Single key 'j' -> ScrollDown
    let j_event = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let action = dispatcher.handle_event(j_event, &keymap);
    assert_eq!(action, Some(KeyAction::ScrollDown));

    // Arrow Down -> ScrollDown
    let down_event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    let action_down = dispatcher.handle_event(down_event, &keymap);
    assert_eq!(action_down, Some(KeyAction::ScrollDown));

    // Arrow Up -> ScrollUp
    let up_event = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    let action_up = dispatcher.handle_event(up_event, &keymap);
    assert_eq!(action_up, Some(KeyAction::ScrollUp));

    // Enter -> Select
    let enter_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let action_enter = dispatcher.handle_event(enter_event, &keymap);
    assert_eq!(action_enter, Some(KeyAction::Select));

    // Esc -> Quit
    let esc_event = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let action_esc = dispatcher.handle_event(esc_event, &keymap);
    assert_eq!(action_esc, Some(KeyAction::Quit));

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

    app.load_book(book).await;
    assert_eq!(app.mode, AppMode::Reader);
    assert_eq!(app.reader_view.scroll_offset, 0);

    // Test scroll actions
    app.handle_action(KeyAction::ScrollDown).await;
    assert_eq!(app.reader_view.scroll_offset, 1);

    app.handle_action(KeyAction::ScrollUp).await;
    assert_eq!(app.reader_view.scroll_offset, 0);

    // Test GotoTop & GotoBottom
    app.handle_action(KeyAction::GotoBottom).await;
    assert!(app.reader_view.scroll_offset > 0);

    app.handle_action(KeyAction::GotoTop).await;
    assert_eq!(app.reader_view.scroll_offset, 0);

    // Test Info modal toggle
    app.handle_action(KeyAction::Info).await;
    assert!(app.reader_view.show_info);
    app.handle_action(KeyAction::Quit).await;
    assert!(!app.reader_view.show_info);

    // Test OpenFile action
    app.handle_action(KeyAction::OpenFile).await;
    assert_eq!(app.mode, AppMode::OpenFileInput);
    app.mode = AppMode::Reader;

    // Test TOC modal toggle
    app.handle_action(KeyAction::Toc).await;
    assert!(app.reader_view.show_toc);

    // Quit while TOC is open closes TOC
    app.handle_action(KeyAction::Quit).await;
    assert!(!app.reader_view.show_toc);
    assert_eq!(app.mode, AppMode::Reader);

    // Quit while in Reader switches to Library
    app.handle_action(KeyAction::Quit).await;
    assert_eq!(app.mode, AppMode::Library);

    // Quit while in Library stops app
    app.handle_action(KeyAction::Quit).await;
    assert!(!app.is_running);

    // Test Widescreen toggle
    app.handle_action(KeyAction::ToggleWidescreen).await;
    assert!(app.config.display.widescreen);
    app.handle_action(KeyAction::ToggleWidescreen).await;
    assert!(!app.config.display.widescreen);
}
