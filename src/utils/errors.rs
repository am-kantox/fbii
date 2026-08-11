use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Keybinding conflict error: {0}")]
    KeybindingConflict(String),

    #[error("Book parse error: {0}")]
    Parse(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Resource not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
