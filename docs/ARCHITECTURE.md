# System Architecture & Design Specification

`tabook` (Rust edition) is a terminal e-book reader for **FB2**, **FB2.ZIP**, and **EPUB (2.x/3.x)** written in Rust.

## System Components

```
                    ┌─────────────────────────┐
                    │      CLI Framework      │
                    │         (clap)          │
                    └────────────┬────────────┘
                                 │
                 ┌───────────────┴───────────────┐
                 ▼                               ▼
     ┌───────────────────────┐       ┌───────────────────────┐
     │    Config Manager     │       │    Library Database   │
     │   (serde + toml)      │       │     (sqlx + sqlite)   │
     └───────────────────────┘       └───────────────────────┘
                                                 │
                 ┌───────────────────────────────┘
                 ▼
     ┌───────────────────────┐       ┌───────────────────────┐
     │    Format Parsers     │──────►│  Unified Block Model  │
     │  FB2 / FB2.zip / EPUB │       │ (Book / Block / Inline)│
     └───────────────────────┘       └───────────┬───────────┘
                                                 │
                 ┌───────────────────────────────┴───────────────────────────────┐
                 ▼                                                               ▼
     ┌───────────────────────┐                                       ┌───────────────────────┐
     │     Layout Engine     │                                       │  Search & NFKD Index  │
     │ (unicode-width wrap)  │                                       │(unicode-normalization)│
     └───────────┬───────────┘                                       └───────────────────────┘
                 │
                 ▼
     ┌───────────────────────┐
     │    Ratatui TUI App    │
     │ (crossterm + tokio)   │
     └───────────────────────┘
```

## Architectural Layers

1. **CLI Layer (`src/cli/`)**:
   - `clap` struct for flag parsing (`tabook [file]`, `--library`, `--theme`, `--config`).

2. **Configuration (`src/config/`)**:
   - Loads defaults, merges user overrides from `~/.config/tabook/config.toml`.
   - Normalizes keybindings and validates conflict invariants.

3. **Format Parsers (`src/formats/`)**:
   - **`encoding.rs`**: BOM detection & XML charset decoding using `encoding_rs`.
   - **`fb2.rs`**: XML DOM parsing (`roxmltree`), base64 asset extraction, structured sections.
   - **`epub.rs`**: EPUB ZIP archive reading (`zip` crate), OPF manifest/spine parsing, NCX/NAV Table of Contents extraction.
   - **`model.rs`**: Format-neutral document tree (`Book`, `Block`, `Inline`).

4. **Persistence Layer (`src/db/`)**:
   - `sqlx` SQLite pool storing books metadata, char-offset reading progress, bookmarks, session stats, and history.

5. **Layout & Search (`src/renderer/`, `src/search/`)**:
   - `BookLayout`: Wraps text into `WrappedLine` instances adhering to column `measure` with `unicode-width`.
   - `BookSearchIndex`: In-memory NFKD Unicode-normalized search index.

6. **TUI & Event Loop (`src/tui/`, `src/themes/`)**:
   - `Ratatui` + `Crossterm` UI layout.
   - Multi-key Vim sequence resolver (`KeymapDispatcher`).
   - Theme color catalog (`Theme`).
