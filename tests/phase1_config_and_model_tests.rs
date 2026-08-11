use tabook::config::{normalize_key, Config, KeyAction, KeyMap};
use tabook::formats::model::{Block, Inline, ListItem};
use tabook::utils::AppError;

#[test]
fn test_default_config_loading() {
    let config = Config::default();
    assert_eq!(config.theme, "dracula");
    assert_eq!(config.typography.measure, 80);
    assert!(!config.typography.hyphenation);
    assert!(config.display.respect_epub_css);
}

#[test]
fn test_config_clamping_and_normalization() {
    let toml_data = r#"
        theme = "monokai"
        [typography]
        measure = 10  # Should be clamped to 30
        line_spacing = 2
        paragraph_indent = 4
        paragraph_spacing = 1
        hyphenation = true

        [display]
        simplified_mode = true
        respect_epub_css = false
        image_protocol = "kitty"

        [keymap.bindings]
        "Ctrl+d" = "half_page_down"
        "j" = "scroll_down"
    "#;

    let config = Config::load_from_str(toml_data).unwrap();
    assert_eq!(config.theme, "monokai");
    assert_eq!(config.typography.measure, 30); // Clamped to min 30
    assert!(config.typography.hyphenation);
    assert!(config.display.simplified_mode);
    assert_eq!(config.display.image_protocol, "kitty");
}

#[test]
fn test_key_normalization() {
    assert_eq!(normalize_key("Ctrl+d"), "ctrl+d");
    assert_eq!(normalize_key("Alt+Ctrl+f"), "alt+ctrl+f");
    assert_eq!(normalize_key("j"), "j");
}

#[test]
fn test_keybinding_conflict_detection() {
    let mut keymap = KeyMap::default();
    keymap.bindings.insert("j".to_string(), KeyAction::ScrollDown);
    keymap.bindings.insert("J".to_string(), KeyAction::ScrollUp);
    // Bind 'j' again to something else to trigger conflict
    keymap.bindings.insert("j".to_string(), KeyAction::Quit);

    let _res = keymap.validate_and_normalize();
    // Re-inserting in Rust hashmap overwrites, but if two different raw keys normalize to same string:
    let mut conflict_keymap = KeyMap::default();
    conflict_keymap.bindings.clear();
    conflict_keymap.bindings.insert("Ctrl+d".to_string(), KeyAction::HalfPageDown);
    conflict_keymap.bindings.insert("ctrl+d".to_string(), KeyAction::PageDown);

    let conflict_res = conflict_keymap.validate_and_normalize();
    assert!(conflict_res.is_err());
    if let Err(AppError::KeybindingConflict(msg)) = conflict_res {
        assert!(msg.contains("conflict"));
    } else {
        panic!("Expected KeybindingConflict error");
    }
}

#[test]
fn test_document_model_plain_text() {
    let paragraph = Block::Paragraph(vec![
        Inline::Text("Hello ".to_string()),
        Inline::Bold(vec![Inline::Text("world".to_string())]),
        Inline::Text("!".to_string()),
    ]);
    assert_eq!(paragraph.plain_text(), "Hello world!");

    let heading = Block::Heading {
        level: 1,
        inlines: vec![Inline::Text("Chapter 1".to_string())],
    };
    assert_eq!(heading.plain_text(), "Chapter 1");

    let list = Block::List {
        ordered: false,
        items: vec![
            ListItem { inlines: vec![Inline::Text("Item 1".to_string())] },
            ListItem { inlines: vec![Inline::Text("Item 2".to_string())] },
        ],
    };
    assert_eq!(list.plain_text(), "Item 1\nItem 2");
}

#[test]
fn test_config_toml_roundtrip() {
    let original = Config::default();
    let toml_str = original.serialize_to_toml().unwrap();
    let reloaded = Config::load_from_str(&toml_str).unwrap();
    assert_eq!(original.theme, reloaded.theme);
    assert_eq!(original.typography.measure, reloaded.typography.measure);
}
