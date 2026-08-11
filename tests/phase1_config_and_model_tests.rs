use tabook::config::{Config, KeyAction, KeyMap};
use tabook::formats::model::{Block, Inline, ListItem, PoemStanza, TableCell, TableRow};
use tabook::utils::AppError;

#[test]
fn test_default_config_loading() {
    let config = Config::default();
    assert_eq!(config.theme, "dracula");
    assert_eq!(config.typography.measure, 80);
    assert_eq!(config.typography.paragraph_indent, 2);
    assert!(!config.display.simplified_mode);
}

#[test]
fn test_config_clamping_and_normalization() {
    let mut config = Config::default();
    config.typography.measure = 10; // Below min 30
    config.clamp_and_validate().unwrap();
    assert_eq!(config.typography.measure, 30);

    config.typography.measure = 500; // Above max 200
    config.clamp_and_validate().unwrap();
    assert_eq!(config.typography.measure, 200);
}

#[test]
fn test_config_toml_roundtrip() {
    let config = Config {
        theme: "monokai".to_string(),
        typography: tabook::config::TypographyConfig {
            measure: 75,
            ..Default::default()
        },
        ..Default::default()
    };

    let toml_str = toml::to_string(&config).unwrap();
    assert!(toml_str.contains("monokai"));

    let loaded: Config = toml::from_str(&toml_str).unwrap();
    assert_eq!(loaded.theme, "monokai");
    assert_eq!(loaded.typography.measure, 75);
}

#[test]
fn test_key_normalization() {
    assert_eq!(tabook::config::keymap::normalize_key("CTRL+d"), "ctrl+d");
    assert_eq!(
        tabook::config::keymap::normalize_key("Ctrl+Alt+h"),
        "alt+ctrl+h"
    );
    assert_eq!(tabook::config::keymap::normalize_key("j"), "j");
    assert_eq!(tabook::config::keymap::normalize_key("Enter"), "Enter");
    assert_eq!(tabook::config::keymap::normalize_key("Esc"), "Esc");
}

#[test]
fn test_keybinding_conflict_detection() {
    let mut keymap = KeyMap::default();
    keymap
        .bindings
        .insert("j".to_string(), KeyAction::ScrollDown);
    keymap.bindings.insert("J".to_string(), KeyAction::ScrollUp);
    keymap.bindings.insert("j".to_string(), KeyAction::Quit);

    let mut conflict_keymap = KeyMap::default();
    conflict_keymap.bindings.clear();
    conflict_keymap
        .bindings
        .insert("Ctrl+d".to_string(), KeyAction::HalfPageDown);
    conflict_keymap
        .bindings
        .insert("ctrl+d".to_string(), KeyAction::PageDown);

    let conflict_res = conflict_keymap.validate_and_normalize();
    assert!(conflict_res.is_err());
}

#[test]
fn test_document_model_plain_text() {
    let inline = Inline::Bold(vec![
        Inline::Text("Hello ".to_string()),
        Inline::Italic(vec![Inline::Text("World".to_string())]),
    ]);
    assert_eq!(inline.plain_text(), "Hello World");

    let list = Block::List {
        ordered: false,
        items: vec![
            ListItem {
                inlines: vec![Inline::Text("Item 1".to_string())],
            },
            ListItem {
                inlines: vec![Inline::Text("Item 2".to_string())],
            },
        ],
    };
    assert_eq!(list.plain_text(), "Item 1\nItem 2");
}

#[test]
fn test_block_plain_text_all_variants() {
    let header = Block::Heading {
        level: 1,
        inlines: vec![Inline::Text("Chapter One".to_string())],
    };
    assert_eq!(header.plain_text(), "Chapter One");

    let quote = Block::Quote(vec![Block::Paragraph(vec![Inline::Text(
        "To be or not to be".to_string(),
    )])]);
    assert_eq!(quote.plain_text(), "To be or not to be");

    let poem = Block::Poem {
        stanzas: vec![PoemStanza {
            lines: vec![vec![Inline::Text("Roses are red".to_string())]],
        }],
    };
    assert_eq!(poem.plain_text(), "Roses are red");

    let table = Block::Table {
        rows: vec![TableRow {
            cells: vec![TableCell {
                inlines: vec![Inline::Text("Cell 1".to_string())],
                is_header: false,
            }],
        }],
    };
    assert_eq!(table.plain_text(), "Cell 1");

    let empty = Block::Empty;
    assert_eq!(empty.plain_text(), "");
}

#[test]
fn test_app_error_formatting() {
    let err_io = AppError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert!(format!("{}", err_io).contains("file not found"));

    let err_parse = AppError::Parse("Invalid XML".to_string());
    assert!(format!("{}", err_parse).contains("Invalid XML"));

    let err_key = AppError::KeybindingConflict("j conflict".to_string());
    assert!(format!("{}", err_key).contains("j conflict"));
}
