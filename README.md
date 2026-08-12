# FBII (Rust Port)

**`fbii`** is a high-performance terminal e-book reader for **FB2**, **FB2-in-ZIP**, and **EPUB (2.x / 3.x)** written in Rust with Vim-like navigation controls.

## Features

- **Format Parsing**: Seamless parsing of `.fb2`, `.fb2.zip`, and `.epub` archives.
- **Auto Character Encoding**: Auto-detects XML charset declarations and BOMs (UTF-8, Windows-1251, ISO-8859-1) via `encoding_rs`.
- **Database Storage**: Async SQLite library management (`sqlx`) tracking reading history, progress, bookmarks, and reading sessions.
- **Library Management**: Recursively scan a directory to bulk-import books (`:scan <dir>` or `--scan-dir`), delete books/bookmarks, sort the library (recent/title/author), and live-filter the library list.
- **Layout & Unicode Wrapping**: `unicode-width` line wrapping with custom measure limits, configurable line/paragraph spacing, and hyphenation.
- **Full-Text Search Index**: Fast NFKD Unicode-normalized search (stripping accents & diacritics).
- **Inline Image Viewing**: Renders embedded FB2/EPUB images using whatever terminal graphics protocol is available (Kitty, iTerm2, Sixel, or Unicode half-blocks), via `ratatui-image`.
- **OPDS Catalogs**: Browse and download books from OPDS feeds; custom catalogs are persisted to the config file.
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

# Recursively import every book found in a directory, then start normally
cargo run -- --scan-dir ~/Books
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
| `Ctrl+f` / `Ctrl+b` | Scroll down / up 1 page (matches your terminal's height) |
| `Ctrl+d` | Scroll down 1/2 page |
| `Ctrl+u` | Scroll up 1/2 page |
| `gg` | Jump to top of document |
| `G` | Jump to bottom of document |
| `/` | Start full-text search (in Reader) / filter the library list (in Library) |
| `n` / `N` | Next / Previous search match |
| `t` | Open Table of Contents modal |
| `b` | Add bookmark |
| `B` | List bookmarks modal |
| `d` | Delete selected book (Library) / delete selected bookmark (Bookmarks modal) |
| `r` | Cycle library sort order (Recent / Title / Author) |
| `v` | View the image on the current line, if any |
| `S` | Toggle Simplified Mode |
| `?` | Help overlay |
| `q` | Quit / Back to library |

## Command Mode

Press `:` to enter command mode. Supported commands include:

| Command | Action |
| :--- | :--- |
| `:open <path\|url>` (or `:o`) | Open a local file, `file://` URI, or `http(s)://` URL |
| `:scan <dir>` | Recursively import every `.fb2`/`.fb2.zip`/`.epub` file found under `<dir>` |
| `:save` (or `:w`) | Save reading progress to the library |
| `:bookmark` (or `:b`) | Add a bookmark at the current line |
| `:bookmarks` (or `:bl`) | Show the bookmarks modal |
| `:toc` | Show the Table of Contents modal |
| `:info` | Show book metadata |
| `:theme <name>` | Switch theme |
| `:themes` | Open the theme picker |
| `:opds` | Open the default OPDS catalog (Project Gutenberg) |
| `:opds add <name> <url>` | Add and persist a custom OPDS catalog |
| `:opds open <name\|url>` | Open a named catalog (or an arbitrary URL) |
| `:goto <line>` (or a bare number) | Jump to a specific line |
| `:config` | Open `config.toml` in `$EDITOR`/`$VISUAL` (falls back to `nano`) |
| `:quit` (or `:q`) | Back to library / quit |
| `:quitall` (or `:qa`) | Quit immediately |

## Configuration

On first run, `fbii` uses built-in defaults; running `:config` creates and opens
`~/.config/fbii/config.toml` (or the path given via `--config`). Example:

```toml
theme = "nord"
db_path = "~/.config/fbii/library.db"

[typography]
measure = 80          # max line width in columns
line_spacing = 1      # blank lines inserted between wrapped lines
paragraph_indent = 2
paragraph_spacing = 1
hyphenation = true
justified = false

[display]
simplified_mode = false
respect_epub_css = true
image_protocol = "auto"  # auto | kitty | iterm2 | sixel | halfblocks | none
widescreen = false

[opds_catalogs]
gutenberg = "https://www.gutenberg.org/ebooks/search.opds/"

[keymap.bindings]
# "j" = "scroll_down", etc. — see the default bindings above.
```

`image_protocol` controls how embedded images are rendered when pressing `v`:
`auto` detects the terminal's capability (Kitty/iTerm2/Sixel, falling back to
Unicode half-blocks); set an explicit value to override detection, or `none`
to disable image rendering entirely.
