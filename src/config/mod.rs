pub mod defaults;
pub mod keymap;

pub use defaults::{DisplayConfig, TypographyConfig};
pub use keymap::{KeyAction, KeyMap, normalize_key};

use crate::utils::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub theme: String,
    pub typography: TypographyConfig,
    pub display: DisplayConfig,
    pub db_path: Option<String>,
    pub keymap: KeyMap,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "dracula".to_string(),
            typography: TypographyConfig::default(),
            display: DisplayConfig::default(),
            db_path: None,
            keymap: KeyMap::default(),
        }
    }
}

impl Config {
    pub fn load_from_str(toml_str: &str) -> Result<Self> {
        let mut config: Config = toml::from_str(toml_str)
            .map_err(|e| AppError::Config(format!("Failed to parse TOML config: {}", e)))?;
        config.clamp_and_validate()?;
        Ok(config)
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    pub fn default_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("tabook").join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn clamp_and_validate(&mut self) -> Result<()> {
        // Clamp measure between 30 and 200
        if self.typography.measure < 30 {
            self.typography.measure = 30;
        } else if self.typography.measure > 200 {
            self.typography.measure = 200;
        }

        // Validate keybindings
        self.keymap.validate_and_normalize()?;

        Ok(())
    }

    pub fn serialize_to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| AppError::Config(format!("Failed to serialize TOML config: {}", e)))
    }
}
