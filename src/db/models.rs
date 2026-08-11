use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct DbBook {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub authors: String, // Comma-separated or JSON
    pub series_name: Option<String>,
    pub series_index: Option<i64>,
    pub genres: String,
    pub annotation: Option<String>,
    pub cover_image_key: Option<String>,
    pub format: String,
    pub progress_offset: i64,
    pub progress_percent: f64,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct DbBookmark {
    pub id: String,
    pub book_id: String,
    pub char_offset: i64,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, FromRow)]
pub struct DbReadingSession {
    pub id: String,
    pub book_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub pages_read: i64,
}
