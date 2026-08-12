pub mod models;

pub use models::{DbBook, DbBookmark, DbReadingSession};

use crate::formats::model::Book;
use crate::utils::{sha1_hex, AppError, Result};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct LibraryDb {
    pool: SqlitePool,
}

impl LibraryDb {
    pub async fn new_in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(AppError::Database)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.init_schema().await?;
        Ok(db)
    }

    pub async fn new_at_path(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let db_url = format!("sqlite://{}", path.to_string_lossy());
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(AppError::Database)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.init_schema().await?;
        Ok(db)
    }

    pub async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY NOT NULL,
                file_path TEXT UNIQUE NOT NULL,
                title TEXT NOT NULL,
                authors TEXT NOT NULL,
                series_name TEXT,
                series_index INTEGER,
                genres TEXT NOT NULL,
                annotation TEXT,
                cover_image_key TEXT,
                format TEXT NOT NULL,
                progress_offset INTEGER NOT NULL DEFAULT 0,
                progress_percent REAL NOT NULL DEFAULT 0.0,
                added_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            );

            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY NOT NULL,
                book_id TEXT NOT NULL,
                char_offset INTEGER NOT NULL,
                label TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS reading_sessions (
                id TEXT PRIMARY KEY NOT NULL,
                book_id TEXT NOT NULL,
                start_time DATETIME NOT NULL,
                end_time DATETIME,
                pages_read INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                book_id TEXT NOT NULL,
                opened_at DATETIME NOT NULL,
                FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_book(
        &self,
        book: &Book,
        progress_offset: usize,
        progress_percent: f64,
    ) -> Result<()> {
        let now = Utc::now();
        let authors_str = book.metadata.authors.join(", ");
        let genres_str = book.metadata.genres.join(", ");
        let format_str = book.metadata.format.to_string();

        sqlx::query(
            r#"
            INSERT INTO books (
                id, file_path, title, authors, series_name, series_index,
                genres, annotation, cover_image_key, format,
                progress_offset, progress_percent, added_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO UPDATE SET
                file_path = excluded.file_path,
                title = excluded.title,
                authors = excluded.authors,
                series_name = excluded.series_name,
                series_index = excluded.series_index,
                genres = excluded.genres,
                annotation = excluded.annotation,
                cover_image_key = excluded.cover_image_key,
                progress_offset = excluded.progress_offset,
                progress_percent = excluded.progress_percent,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&book.id)
        .bind(&book.file_path)
        .bind(&book.metadata.title)
        .bind(&authors_str)
        .bind(&book.metadata.series_name)
        .bind(book.metadata.series_index.map(|i| i as i64))
        .bind(&genres_str)
        .bind(&book.metadata.annotation)
        .bind(&book.metadata.cover_image_key)
        .bind(&format_str)
        .bind(progress_offset as i64)
        .bind(progress_percent)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_books(&self) -> Result<Vec<DbBook>> {
        let books = sqlx::query_as::<_, DbBook>(
            "SELECT id, file_path, title, authors, series_name, series_index, genres, annotation, cover_image_key, format, progress_offset, progress_percent, added_at, updated_at FROM books ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(books)
    }

    pub async fn get_book_by_id(&self, id: &str) -> Result<Option<DbBook>> {
        let book = sqlx::query_as::<_, DbBook>(
            "SELECT id, file_path, title, authors, series_name, series_index, genres, annotation, cover_image_key, format, progress_offset, progress_percent, added_at, updated_at FROM books WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(book)
    }

    pub async fn get_book_by_path(&self, path: &str) -> Result<Option<DbBook>> {
        let book = sqlx::query_as::<_, DbBook>(
            "SELECT id, file_path, title, authors, series_name, series_index, genres, annotation, cover_image_key, format, progress_offset, progress_percent, added_at, updated_at FROM books WHERE file_path = ?1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(book)
    }

    pub async fn delete_book(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM books WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn add_bookmark(
        &self,
        book_id: &str,
        char_offset: usize,
        label: &str,
    ) -> Result<DbBookmark> {
        let id = sha1_hex(format!("{}:{}:{}", book_id, char_offset, label).as_bytes());
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO bookmarks (id, book_id, char_offset, label, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&id)
        .bind(book_id)
        .bind(char_offset as i64)
        .bind(label)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(DbBookmark {
            id,
            book_id: book_id.to_string(),
            char_offset: char_offset as i64,
            label: label.to_string(),
            created_at: now,
        })
    }

    pub async fn list_bookmarks(&self, book_id: &str) -> Result<Vec<DbBookmark>> {
        let bookmarks = sqlx::query_as::<_, DbBookmark>(
            "SELECT id, book_id, char_offset, label, created_at FROM bookmarks WHERE book_id = ?1 ORDER BY char_offset ASC",
        )
        .bind(book_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(bookmarks)
    }

    pub async fn delete_bookmark(&self, bookmark_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM bookmarks WHERE id = ?1")
            .bind(bookmark_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn record_history(&self, book_id: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query("INSERT INTO history (book_id, opened_at) VALUES (?1, ?2)")
            .bind(book_id)
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn start_reading_session(&self, book_id: &str) -> Result<String> {
        let id = sha1_hex(
            format!(
                "{}:{}",
                book_id,
                Utc::now().timestamp_nanos_opt().unwrap_or(0)
            )
            .as_bytes(),
        );
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO reading_sessions (id, book_id, start_time, pages_read) VALUES (?1, ?2, ?3, 0)",
        )
        .bind(&id)
        .bind(book_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn end_reading_session(&self, session_id: &str, pages_read: u32) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE reading_sessions SET end_time = ?1, pages_read = ?2 WHERE id = ?3")
            .bind(now)
            .bind(pages_read as i64)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
