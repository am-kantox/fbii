# FBII (Rust Port)

**`fbii`** is a high-performance terminal e-book reader for **FB2**, **FB2-in-ZIP**, and **EPUB (2.x / 3.x)** written in Rust with Vim-like navigation controls.

## Features

- **Format Parsing**: Seamless parsing of `.fb2`, `.fb2.zip`, and `.epub` archives.
- **Auto Character Encoding**: Auto-detects XML charset declarations and BOMs (UTF-8, Windows-1251, ISO-8859-1) via `encoding_rs`.
- **Database Storage**: Async SQLite library management (`sqlx`) tracking reading history, progress, and bookmarks.
- **Layout & Unicode Wrapping**: `unicode-width` line wrapping with custom measure limits.
- **Full-Text Search Index**: Fast NFKD Unicode-normalized search (stripping accents & diacritics).
- **Themes**: Built-in color palettes (`dracula`, `monokai`, `github-dark`, `github-light`).

## Quick Start

### Build & Run

```bash
# Build debug binary
cargo build

# Open an e-book file
cargo run -- /path/to/book.epub

# Launch library view
cargo run -- --library

# Custom theme
cargo run -- --theme monokai /path/to/book.fb2

# Specify config file
cargo run -- --config ~/.config/fbii/config.toml
```

### Installation

```bash
cargo install --path .
```

## Vim Navigation Keybindings

| Keybinding | Action |
| :--- | :--- |
| `j` / `Down` | Scroll down 1 line |
| `k` / `Up` | Scroll up 1 line |
| `Ctrl+d` | Scroll down 1/2 page |
| `Ctrl+u` | Scroll up 1/2 page |
| `gg` | Jump to top of document |
| `G` | Jump to bottom of document |
| `/` | Start full-text search |
| `n` / `N` | Next / Previous search match |
| `t` | Open Table of Contents modal |
| `b` | Add bookmark |
| `B` | List bookmarks modal |
| `S` | Toggle Simplified Mode |
| `?` | Help overlay |
| `q` | Quit / Back to library |
