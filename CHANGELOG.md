# Changelog

All notable changes to this project are documented in this file.

## Unreleased

### Added (second pass)

- Images are now rendered inline in the scrollable text flow (reserving real space in the layout) instead of only as a text placeholder; `v` remains available to zoom an image to full size in a modal.
- Book cover art is decoded and displayed in the Info modal (`i`), using the same terminal graphics protocol as inline images.
- Reading-session stats (session count, total pages read) are now surfaced in the Info modal, using data that was previously recorded but never shown anywhere.
- Inline `display: none` CSS in EPUB content is now detected at parse time and hidden by default; `C` toggles whether it is shown, finally giving that setting (and keybinding) a real effect.
- Word wrapping now uses Unicode word boundaries with grapheme-level sub-splitting for wide characters, so space-free scripts (CJK, etc.) wrap correctly character-by-character instead of only ever splitting once via hyphenation.
- Deleting a library book or bookmark (`d`) now requires a second confirming press; any other keypress in between cancels the pending deletion.
- Added `ratatui::backend::TestBackend`-based rendering tests for `LibraryView`/`ReaderView`, plus unit tests for `themes::catalog`, `opds::model`, and the new inline-image run-detection logic.

### Changed (second pass)

- `BookLayout::line_at_char_offset` now uses a binary search (`partition_point`) instead of a linear scan.
- Trimmed `ratatui-image`/`image` codec features to just the formats useful for e-book cover art (JPEG, PNG, GIF, WebP), instead of pulling in the full default codec set.

### Fixed

- Book identity is now derived from the canonicalized file path instead of file content (FB2) or bare filename (EPUB), so external edits no longer orphan reading progress/bookmarks, and same-named books in different directories no longer collide.
- Reading position is preserved (by character offset) when toggling simplified mode or text justification, instead of jumping to an arbitrary line.
- Reading progress percentage is now computed with a single shared formula, used consistently by the status bar and by the library save path.
- Page/half-page scrolling now uses the actual rendered viewport height instead of hardcoded line counts.
- Fixed a boundary tie-break bug in `BookLayout::line_at_char_offset` that could resolve a char offset landing exactly on a shared line boundary to the wrong (earlier) line.
- Non-ASCII (percent-encoded) file paths passed via `file://` URIs are decoded as whole UTF-8 byte sequences instead of being mangled by a per-byte `char` cast.
- Network requests (OPDS feed fetches, remote book downloads) now use a shared HTTP client with connect/request timeouts, instead of hanging indefinitely on a slow or dead server.
- Opening a book from the library that fails to parse now surfaces the error in the status bar instead of failing silently.
- The terminal is now restored (raw mode disabled, alternate screen exited) via a panic hook if the app panics mid-render, instead of leaving the terminal unusable.
- `:config edit` now respects a custom `--config` path instead of always writing to the default config location.

### Added

- `:scan <dir>` command and `--scan-dir` CLI flag to recursively import e-book files from a directory without resetting progress on already-known books.
- Library management: delete the selected book (`d`) or bookmark (`d` in the bookmarks modal), cycle sort order (`r`: recent/title/author), and live-filter the library list (`/`).
- Inline image viewing (`v`) using `ratatui-image`, supporting Kitty, iTerm2, Sixel, and Unicode half-block protocols, configurable via `display.image_protocol`.
- `typography.line_spacing` is now applied when rendering (previously accepted but ignored).
- OPDS catalogs added via `:opds add` are now persisted to `config.toml`.
- Reading history and reading-session tracking (previously implemented in the database layer but never invoked) are now recorded automatically as books are opened and closed.
- `LICENSE` file (MIT, matching `Cargo.toml`).
- Expanded automated test coverage for the new/fixed functionality.

### Changed

- Consolidated three duplicate SHA1-hashing helpers (each misleadingly named `md5_hash`) into a single `utils::sha1_hex`.
- De-duplicated the sync/async remote-download and local-path-resolution logic in `formats::parse_book_uri`/`parse_book_uri_async`.
- Split the monolithic `App::run_tui` input-handling loop into dedicated per-mode handler methods.
- `reqwest` now uses `rustls-tls` instead of the platform TLS backend for easier cross-compilation.
- Trimmed `tokio`'s enabled features to what the crate actually uses.
- Added `[profile.release]` tuning (`lto`, `strip`, `codegen-units = 1`).
