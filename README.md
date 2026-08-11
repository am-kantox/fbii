# tabook (Rust Port)

A high-performance terminal e-book reader for **FB2**, **FB2-in-ZIP**, and **EPUB (2.x / 3.x)** with vim-like controls, built in **Rust** using **Ratatui**, **Crossterm**, **Tokio**, and **SQLx**.

## Features

- **Format Support**: FB2, FB2.zip, EPUB 2.x/3.x with inline formatting, tables, poems, quotes, and images.
- **Vim-like Controls**: `j`/`k`, `gg`/`G`, `Ctrl+D`/`Ctrl+U`, `/` search, `:` command prompt, and customizable keymaps.
- **Library & Progress**: SQLite database tracking reading position, progress percentage, bookmarks, and sessions.
- **Typography & Themes**: Adjustable line measure, spacing, indentation, soft hyphenation, and themes (Dracula, Monokai, Ayu, GitHub, Gruvbox, Nord, Solarized, Catppuccin).
- **Terminal Graphics**: Book cover and illustration rendering via Kitty Graphics Protocol, Sixel, iTerm2, or half-block text fallbacks.

## Installation & Build

```bash
cargo build --release
```

## CLI Usage

```bash
# Open library view
tabook --library

# Open a specific e-book file
tabook /path/to/book.epub
tabook /path/to/book.fb2

# Override theme
tabook --theme dracula /path/to/book.epub

# Custom config file
tabook --config ~/.config/tabook/config.toml
```

## Documentation

- [Configuration Guide](docs/CONFIGURATION.md)
