<p align="center">
  <img src="images/logo-silver-500x500.png" alt="FBII logo" width="200">
</p>

# FBII

**`fbii`** is a high-performance terminal e-book reader for **FB2**, **FB2-in-ZIP**, and **EPUB (2.x / 3.x)** written in Rust with Vim-like navigation controls. It is a Rust port of [`tabook`](https://github.com/zsh-ncursed/tabook) with some opinionated goodnesses added (like inplace image rendering.)

## Features

- **Format Parsing**: Seamless parsing of `.fb2`, `.fb2.zip`, and `.epub` archives.
- **Auto Character Encoding**: Auto-detects XML charset declarations and BOMs (UTF-8, Windows-1251, ISO-8859-1) via `encoding_rs`.
- **Database Storage**: Async SQLite library management (`sqlx`) tracking reading history, progress, bookmarks, and reading sessions.
- **Library Management**: Recursively scan a directory to bulk-import books (`:scan <dir>` or `--scan-dir`), delete books/bookmarks, sort the library (recent/title/author), and live-filter the library list.
- **Layout & Unicode Wrapping**: `unicode-width` line wrapping with custom measure limits, configurable line/paragraph spacing, and hyphenation.
- **Full-Text Search Index**: Fast NFKD Unicode-normalized search (stripping accents & diacritics), with correct word wrapping for space-free scripts (CJK, etc.).
- **Inplace Image Rendering**: Images are rendered directly inside the scrollable text layout at their exact reading position using Kitty, Sixel, iTerm2, or Unicode half-blocks; press `v` to zoom any image to full size. Book cover art is also displayed in the Info modal (`i`).
- **CSS-Aware EPUB Parsing**: Content marked with an inline `display: none` style is hidden by default; toggle `C` to reveal it.
- **Reading Stats**: Session count and total pages read are tracked per book and shown in the Info modal.
- **OPDS Catalogs**: Browse and download books from OPDS feeds; custom catalogs are persisted to the config file.
- **Themes**: Built-in color palettes (`dracula`, `monokai`, `github-dark`, `github-light`).

## Installation

### Option 1: Prebuilt binary (no Rust toolchain required)

Every [tagged release](../../releases) publishes ready-to-run archives for
Linux (x86_64), macOS (Intel and Apple Silicon), and Windows (x86_64), built
by the [release workflow](.github/workflows/release.yml). To install:

1. Open the [Releases page](../../releases) and download the archive that
   matches your platform (e.g. `fbii-<version>-x86_64-unknown-linux-gnu.tar.gz`,
   `fbii-<version>-aarch64-apple-darwin.tar.gz`, or
   `fbii-<version>-x86_64-pc-windows-msvc.zip`).
2. Extract it (this creates a directory named after the archive):
   ```bash
   tar xzf fbii-*.tar.gz && cd fbii-*/    # Linux / macOS
   # or unzip fbii-*.zip on Windows, then open the extracted folder
   ```
3. Run the extracted binary directly, or move it onto your `PATH`:
   ```bash
   ./fbii --help
   install -m 755 fbii ~/.local/bin/fbii   # optional: install onto PATH
   ```

No system dependencies are required: SQLite is statically compiled into the
binary, and TLS uses `rustls` rather than a system OpenSSL library.

### Option 2: Build from source (requires Rust)

Requires a [Rust toolchain](https://rustup.rs) (stable channel).

```bash
git clone <this-repository-url>
cd fbii
cargo install --path .
```

This installs the `fbii` binary into `~/.cargo/bin` (make sure that directory
is on your `PATH`).

## Usage

### Quick Start

```bash
# Open an e-book file
fbii /path/to/book.epub

# Launch library view
fbii --library

# Custom theme & explicit image protocol
fbii --theme monokai --image-protocol kitty /path/to/book.epub

# Specify config file
fbii --config ~/.config/fbii/config.toml

# Recursively import every book found in a directory, then start normally
fbii --scan-dir ~/Books
```

When running from a source checkout without installing, substitute
`cargo run --` for `fbii` in the examples above (e.g.
`cargo run -- --library`).

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
| `d` | Delete selected book (Library) / delete selected bookmark (Bookmarks modal); press again to confirm |
| `r` | Cycle library sort order (Recent / Title / Author) |
| `v` | Zoom the image on the current line to full size |
| `i` | Show book info (title, metadata, cover art, reading stats) |
| `S` | Toggle Simplified Mode |
| `C` | Toggle whether CSS-hidden (`display: none`) EPUB content is shown |
| `?` | Help overlay |
| `q` | Quit / Back to library |

Deleting a book or bookmark requires two presses of `d` in a row; any other
keypress in between cancels the pending confirmation.

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
| `:protocol <auto\|kitty\|sixel\|iterm2\|halfblocks\|none>` | Switch image graphics protocol dynamically |
| `:theme <name>` | Switch theme |
| `:themes` | Open the theme picker |
| `:opds` | Open the default OPDS catalog (Project Gutenberg) |
| `:opds add <name> <url>` | Add and persist a custom OPDS catalog |
| `:opds open <name\|url>` | Open a named catalog (or an arbitrary URL) |
| `:goto <line>` (or a bare number) | Jump to a specific line |
| `:config` | Open `config.toml` in `$EDITOR`/`$VISUAL` (falls back to `nano`) |
| `:quit` (or `:q`) | Back to library / quit |
| `:quitall` (or `:qa`) | Quit immediately |

## Inplace Image Rendering & Terminal Graphics Compatibility

`fbii` features native **inplace inline image rendering**: images inside EPUB and FB2 books are rendered directly inside the scrollable text layout at their exact position in the book, reserving vertical viewport space in real-time. Pressing `v` opens a high-resolution full-screen modal zoom viewer.

### Terminal Graphics Compatibility Guide

`fbii` supports high-resolution pixel rendering protocols as well as universal ANSI block fallbacks:

| Protocol | Config Value | Rendering Description | Compatible Terminal Emulators |
| :--- | :--- | :--- | :--- |
| **Kitty Graphics** | `kitty` | High-resolution native pixel graphics | Kitty, Ghostty, WezTerm, Alacritty (with graphics plugin), Konsole |
| **Sixel Graphics** | `sixel` | High-resolution bitmap graphics | Foot, XTerm, rxvt-unicode (sixel build), WezTerm, Contour, iTerm2 |
| **iTerm2 Inline Images** | `iterm2` | High-resolution inline graphics | iTerm2, **Warp**, WezTerm, Mintty, Tabby |
| **Unicode Half-Blocks** | `halfblocks` | ANSI-colored block character graphics (`▄` `▀` `█`) | Universal (works in *any* terminal emulator) |
| **Disabled** | `none` | Clean single-line text label placeholder | Any terminal |

### Changing Image Protocols

- **CLI Option**: Override protocol when launching `fbii`:
  ```bash
  fbii --image-protocol kitty /path/to/book.epub
  fbii --image-protocol sixel /path/to/book.fb2
  ```
- **In-App Command**: Switch rendering protocol on the fly while reading:
  ```text
  :protocol kitty
  ```
- **Config File**: Set your default protocol in `~/.config/fbii/config.toml`:
  ```toml
  [display]
  image_protocol = "kitty"  # auto | kitty | sixel | iterm2 | halfblocks | none
  ```

When set to `auto` (default), `fbii` queries the terminal at startup to detect hardware graphics support, falling back to `halfblocks` if the terminal doesn't respond to graphics capability queries.

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
