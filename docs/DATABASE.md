# Database Architecture & Schema

`tabook` uses an embedded **SQLite** database (`sqlx` runtime) located at `~/.config/tabook/library.db` (or custom path specified in `config.toml`).

## Tables

### `books`
Stores metadata and reading position for library books.

```sql
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
```

### `bookmarks`
Stores text bookmarks with labels and exact character offsets.

```sql
CREATE TABLE IF NOT EXISTS bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    book_id TEXT NOT NULL,
    char_offset INTEGER NOT NULL,
    label TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
);
```

### `reading_sessions`
Tracks reading session statistics (start time, end time, pages read).

```sql
CREATE TABLE IF NOT EXISTS reading_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    book_id TEXT NOT NULL,
    start_time DATETIME NOT NULL,
    end_time DATETIME,
    pages_read INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
);
```

### `history`
Maintains recent open history.

```sql
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id TEXT NOT NULL,
    opened_at DATETIME NOT NULL,
    FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
);
```
