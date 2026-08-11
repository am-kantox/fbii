# SQLite Database Layer

`fbii` uses an embedded **SQLite** database (`sqlx` runtime) located at `~/.config/fbii/library.db` (or custom path specified in `config.toml`).

## Schema Design

### `books`
- `id` (TEXT PRIMARY KEY) — SHA1/MD5 hash of book content or path.
- `title` (TEXT NOT NULL)
- `authors` (TEXT NOT NULL) — Comma-separated author list.
- `series_name` (TEXT)
- `series_index` (INTEGER)
- `format` (TEXT NOT NULL) — `fb2`, `fb2.zip`, or `epub`.
- `path` (TEXT NOT NULL UNIQUE)
- `added_at` (DATETIME NOT NULL)
- `last_read_at` (DATETIME)
- `progress_offset` (INTEGER DEFAULT 0) — Reading position character offset.
- `progress_percent` (REAL DEFAULT 0.0) — Percentage progress (0.0 to 100.0).

### `bookmarks`
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `book_id` (TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE)
- `char_offset` (INTEGER NOT NULL)
- `snippet` (TEXT NOT NULL)
- `created_at` (DATETIME NOT NULL)

### `reading_sessions`
- `id` (TEXT PRIMARY KEY)
- `book_id` (TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE)
- `start_time` (DATETIME NOT NULL)
- `end_time` (DATETIME)
- `pages_read` (INTEGER DEFAULT 0)

### `history`
- `id` (INTEGER PRIMARY KEY AUTOINCREMENT)
- `book_id` (TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE)
- `opened_at` (DATETIME NOT NULL)
