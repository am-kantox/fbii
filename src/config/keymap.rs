use crate::utils::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    ScrollDown,
    ScrollUp,
    PageDown,
    PageUp,
    HalfPageDown,
    HalfPageUp,
    GotoTop,
    GotoBottom,
    Search,
    NextMatch,
    PrevMatch,
    OpenFile,
    SaveToLibrary,
    AddBookmark,
    ListBookmarks,
    Toc,
    Info,
    Help,
    Quit,
    Command,
    ToggleSimpleMode,
    ToggleCss,
    Select,
}

impl std::fmt::Display for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            KeyAction::ScrollDown => "scroll_down",
            KeyAction::ScrollUp => "scroll_up",
            KeyAction::PageDown => "page_down",
            KeyAction::PageUp => "page_up",
            KeyAction::HalfPageDown => "half_page_down",
            KeyAction::HalfPageUp => "half_page_up",
            KeyAction::GotoTop => "goto_top",
            KeyAction::GotoBottom => "goto_bottom",
            KeyAction::Search => "search",
            KeyAction::NextMatch => "next_match",
            KeyAction::PrevMatch => "prev_match",
            KeyAction::OpenFile => "open_file",
            KeyAction::SaveToLibrary => "save_to_library",
            KeyAction::AddBookmark => "add_bookmark",
            KeyAction::ListBookmarks => "list_bookmarks",
            KeyAction::Toc => "toc",
            KeyAction::Info => "info",
            KeyAction::Help => "help",
            KeyAction::Quit => "quit",
            KeyAction::Command => "command",
            KeyAction::ToggleSimpleMode => "toggle_simple_mode",
            KeyAction::ToggleCss => "toggle_css",
            KeyAction::Select => "select",
        };
        write!(f, "{}", name)
    }
}

pub fn normalize_key(key: &str) -> String {
    let key = key.trim();
    let parts: Vec<&str> = key.split('+').collect();
    if parts.len() > 1 {
        let mut mods: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|m| m.to_lowercase())
            .collect();
        mods.sort();
        let last = parts.last().unwrap();
        format!("{}+{}", mods.join("+"), last)
    } else {
        key.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyMap {
    pub bindings: HashMap<String, KeyAction>,
}

impl Default for KeyMap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert("j".to_string(), KeyAction::ScrollDown);
        bindings.insert("Down".to_string(), KeyAction::ScrollDown);
        bindings.insert("k".to_string(), KeyAction::ScrollUp);
        bindings.insert("Up".to_string(), KeyAction::ScrollUp);
        bindings.insert("ctrl+f".to_string(), KeyAction::PageDown);
        bindings.insert("PageDown".to_string(), KeyAction::PageDown);
        bindings.insert("ctrl+b".to_string(), KeyAction::PageUp);
        bindings.insert("PageUp".to_string(), KeyAction::PageUp);
        bindings.insert("ctrl+d".to_string(), KeyAction::HalfPageDown);
        bindings.insert("ctrl+u".to_string(), KeyAction::HalfPageUp);
        bindings.insert("gg".to_string(), KeyAction::GotoTop);
        bindings.insert("G".to_string(), KeyAction::GotoBottom);
        bindings.insert("/".to_string(), KeyAction::Search);
        bindings.insert("n".to_string(), KeyAction::NextMatch);
        bindings.insert("N".to_string(), KeyAction::PrevMatch);
        bindings.insert("o".to_string(), KeyAction::OpenFile);
        bindings.insert("s".to_string(), KeyAction::SaveToLibrary);
        bindings.insert("b".to_string(), KeyAction::AddBookmark);
        bindings.insert("B".to_string(), KeyAction::ListBookmarks);
        bindings.insert("t".to_string(), KeyAction::Toc);
        bindings.insert("i".to_string(), KeyAction::Info);
        bindings.insert("?".to_string(), KeyAction::Help);
        bindings.insert("q".to_string(), KeyAction::Quit);
        bindings.insert("Esc".to_string(), KeyAction::Quit);
        bindings.insert("Enter".to_string(), KeyAction::Select);
        bindings.insert(":".to_string(), KeyAction::Command);
        bindings.insert("S".to_string(), KeyAction::ToggleSimpleMode);
        bindings.insert("C".to_string(), KeyAction::ToggleCss);
        Self { bindings }
    }
}

impl KeyMap {
    pub fn validate_and_normalize(&mut self) -> Result<()> {
        let mut normalized_map: HashMap<String, KeyAction> = HashMap::new();
        let mut key_to_action: HashMap<String, KeyAction> = HashMap::new();

        for (raw_key, action) in self.bindings.drain() {
            let norm = normalize_key(&raw_key);
            if let Some(existing_action) = key_to_action.get(&norm) {
                if *existing_action != action {
                    return Err(AppError::KeybindingConflict(format!(
                        "Key binding conflict for '{}': bound to '{}' and '{}'",
                        norm, existing_action, action
                    )));
                }
            }
            key_to_action.insert(norm.clone(), action);
            normalized_map.insert(norm, action);
        }

        self.bindings = normalized_map;
        Ok(())
    }
}
