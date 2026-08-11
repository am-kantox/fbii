use crate::config::{KeyAction, KeyMap};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Default)]
pub struct KeymapDispatcher {
    pending_buffer: String,
}

impl KeymapDispatcher {
    pub fn new() -> Self {
        Self {
            pending_buffer: String::new(),
        }
    }

    pub fn handle_event(&mut self, event: KeyEvent, keymap: &KeyMap) -> Option<KeyAction> {
        let key_str = format_key_event(event);
        self.pending_buffer.push_str(&key_str);

        // Check if pending_buffer matches any configured key binding
        if let Some(&action) = keymap.bindings.get(&self.pending_buffer) {
            self.pending_buffer.clear();
            return Some(action);
        }

        // Check if any binding starts with pending_buffer
        let is_prefix = keymap
            .bindings
            .keys()
            .any(|k| k.starts_with(&self.pending_buffer));

        if !is_prefix {
            // Check if single key_str matches directly
            if let Some(&action) = keymap.bindings.get(&key_str) {
                self.pending_buffer.clear();
                return Some(action);
            }
            self.pending_buffer.clear();
        }

        None
    }
}

pub fn format_key_event(event: KeyEvent) -> String {
    let mut mods = Vec::new();
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        mods.push("ctrl");
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        mods.push("alt");
    }

    let code_str = match event.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        _ => "".to_string(),
    };

    if !mods.is_empty() {
        format!("{}+{}", mods.join("+"), code_str.to_lowercase())
    } else {
        code_str
    }
}
