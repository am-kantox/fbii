use crate::config::{Config, KeyAction};
use crate::db::{DbBook, LibraryDb};
use crate::formats::model::Book;
use crate::renderer::BookLayout;
use crate::search::{BookSearchIndex, SearchMatch};
use crate::themes::Theme;
use crate::tui::keymap_dispatcher::KeymapDispatcher;
use crate::tui::views::library_view::LibraryView;
use crate::tui::views::reader_view::ReaderView;
use crate::utils::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui_image::picker::{Picker, ProtocolType};
use std::io::stdout;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Library,
    Reader,
    SearchInput,
    CommandInput,
    OpenFileInput,
    OpdsBrowser,
    /// Live text-filter editing for the library list (distinct from
    /// `SearchInput`, which searches within the currently open book).
    LibraryFilterInput,
}

pub struct App {
    pub mode: AppMode,
    pub config: Config,
    /// Filesystem path the active config was loaded from (or would be saved
    /// to by default). Used so `:config edit` and OPDS catalog persistence
    /// respect a custom `--config` path instead of always writing to the
    /// default location.
    pub config_path: PathBuf,
    pub theme: Theme,
    pub db: LibraryDb,
    pub library_books: Vec<DbBook>,
    pub active_book: Option<Book>,
    pub active_layout: Option<BookLayout>,
    pub search_index: Option<BookSearchIndex>,
    pub search_matches: Vec<SearchMatch>,
    pub current_match_idx: usize,
    pub input_buffer: String,
    pub bookmarks: Vec<crate::db::DbBookmark>,
    pub key_dispatcher: KeymapDispatcher,
    pub library_view: LibraryView,
    pub reader_view: ReaderView,
    pub opds_view: crate::tui::views::OpdsView,
    pub opds_feed_stack: Vec<crate::opds::OpdsFeed>,
    pub opds_catalogs: std::collections::HashMap<String, String>,
    pub status_message: Option<String>,
    pub command_history: Vec<String>,
    pub search_history: Vec<String>,
    pub command_history_idx: Option<usize>,
    pub search_history_idx: Option<usize>,
    pub is_running: bool,
    /// Terminal image picker/protocol, detected (or overridden by
    /// `DisplayConfig::image_protocol`) once the alternate screen is
    /// entered. `None` means image rendering is unavailable or disabled.
    pub image_picker: Option<Picker>,
    /// Id of the in-progress `reading_sessions` row for the active book, if
    /// any.
    pub active_session_id: Option<String>,
    /// Approximate number of "pages" turned during the current reading
    /// session (incremented on page/half-page navigation).
    pub session_pages_read: u32,
}

impl App {
    pub fn new(config: Config, db: LibraryDb, config_path: PathBuf) -> Self {
        let theme = Theme::get_by_name(&config.theme);
        let opds_catalogs = config.opds_catalogs.clone();

        Self {
            mode: AppMode::Library,
            config,
            config_path,
            theme,
            db,
            library_books: Vec::new(),
            active_book: None,
            active_layout: None,
            search_index: None,
            search_matches: Vec::new(),
            current_match_idx: 0,
            key_dispatcher: KeymapDispatcher::new(),
            library_view: LibraryView::new(),
            reader_view: ReaderView::new(),
            opds_view: crate::tui::views::OpdsView::new(),
            opds_feed_stack: Vec::new(),
            opds_catalogs,
            input_buffer: String::new(),
            bookmarks: Vec::new(),
            status_message: None,
            command_history: Vec::new(),
            search_history: Vec::new(),
            command_history_idx: None,
            search_history_idx: None,
            is_running: true,
            image_picker: None,
            active_session_id: None,
            session_pages_read: 0,
        }
    }

    /// Persist the current in-memory config to `self.config_path`.
    pub fn save_config(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let toml_str = self.config.serialize_to_toml()?;
        std::fs::write(&self.config_path, toml_str)?;
        Ok(())
    }

    /// The library entries currently visible under the active filter/sort,
    /// in display order. Selection indices in `library_view.state` always
    /// refer to positions in this list, not `library_books` directly.
    pub fn visible_library_books(&self) -> Vec<&DbBook> {
        self.library_view.visible_books(&self.library_books)
    }

    pub async fn load_book(&mut self, book: Book) {
        let layout = BookLayout::build(
            &book,
            &self.config.typography,
            self.config.display.simplified_mode,
        );
        let search_index = BookSearchIndex::build(&book);

        let mut saved_offset = 0;
        if let Ok(Some(db_book)) = self.db.get_book_by_id(&book.id).await {
            saved_offset = db_book.progress_offset as usize;
        } else if let Ok(Some(db_book)) = self.db.get_book_by_path(&book.file_path).await {
            // Defensive fallback: covers records keyed under a different id
            // than the current path-derived scheme (e.g. legacy imports).
            saved_offset = db_book.progress_offset as usize;
        }

        // Close out any reading session for the previously active book
        // before switching to the new one.
        self.end_active_session().await;

        let book_id = book.id.clone();
        self.active_book = Some(book);
        self.active_layout = Some(layout);
        self.search_index = Some(search_index);
        self.reader_view.scroll_offset = saved_offset;
        self.reader_view.show_image = false;
        self.reader_view.image_state = None;
        self.mode = AppMode::Reader;

        let _ = self.db.record_history(&book_id).await;
        self.session_pages_read = 0;
        self.active_session_id = self.db.start_reading_session(&book_id).await.ok();
    }

    /// End the current reading session (if any), recording the approximate
    /// number of pages turned.
    async fn end_active_session(&mut self) {
        if let Some(session_id) = self.active_session_id.take() {
            let _ = self
                .db
                .end_reading_session(&session_id, self.session_pages_read)
                .await;
        }
        self.session_pages_read = 0;
    }

    pub async fn refresh_library(&mut self) -> Result<()> {
        self.library_books = self.db.list_books().await?;
        Ok(())
    }

    pub async fn open_opds_url(&mut self, url: &str) -> Result<()> {
        let response = crate::formats::http_client()
            .get(url)
            .send()
            .await
            .map_err(|e| {
                crate::utils::AppError::Parse(format!("Failed to fetch OPDS feed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(crate::utils::AppError::Parse(format!(
                "HTTP error {} fetching OPDS feed",
                response.status()
            )));
        }

        let xml_str = response.text().await.map_err(|e| {
            crate::utils::AppError::Parse(format!("Failed to read OPDS feed: {}", e))
        })?;

        let feed = crate::opds::parse_opds_feed(&xml_str, url)?;
        self.opds_feed_stack.push(feed);
        self.opds_view.state.select(Some(0));
        self.mode = AppMode::OpdsBrowser;
        Ok(())
    }

    pub async fn save_progress(&self) {
        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout) {
            let offset = self.reader_view.scroll_offset;
            let percent = layout.progress_percent(offset, self.reader_view.last_viewport_height);
            let _ = self.db.upsert_book(book, offset, percent).await;
        }
    }

    /// Rebuild the active book's layout (e.g. after toggling simplified
    /// mode or justification) while keeping the reading position anchored
    /// to the same character offset, instead of leaving `scroll_offset` as
    /// a now-meaningless raw line index into the old layout.
    fn rebuild_layout_preserving_position(&mut self) {
        if let Some(book) = &self.active_book {
            let char_offset = self
                .active_layout
                .as_ref()
                .and_then(|l| l.lines.get(self.reader_view.scroll_offset))
                .map(|l| l.char_start)
                .unwrap_or(0);

            let layout = BookLayout::build(
                book,
                &self.config.typography,
                self.config.display.simplified_mode,
            );
            self.reader_view.scroll_offset = layout.line_at_char_offset(char_offset);
            self.active_layout = Some(layout);
        }
    }

    /// Handle the `ViewImage` action: look up the image resource key on the
    /// current line (if any) and, when a terminal graphics protocol is
    /// available, decode and display it in a modal.
    async fn handle_view_image(&mut self) {
        let image_key = self
            .active_layout
            .as_ref()
            .and_then(|l| l.lines.get(self.reader_view.scroll_offset))
            .and_then(|l| l.image_key.clone());

        let Some(key) = image_key else {
            self.status_message = Some("No image on the current line.".to_string());
            return;
        };

        let bytes = self
            .active_book
            .as_ref()
            .and_then(|b| b.resources.get(&key).cloned());

        match (bytes, self.image_picker) {
            (Some(bytes), Some(mut picker)) => match image::load_from_memory(&bytes) {
                Ok(dyn_img) => {
                    let protocol = picker.new_resize_protocol(dyn_img);
                    self.image_picker = Some(picker);
                    self.reader_view.image_state = Some(protocol);
                    self.reader_view.current_image_key = Some(key);
                    self.reader_view.show_image = true;
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to decode image: {}", e));
                }
            },
            (None, _) => {
                self.status_message = Some(format!("Image resource '{}' not found.", key));
            }
            (_, None) => {
                self.status_message = Some(
                    "Image rendering is disabled or unavailable in this terminal.".to_string(),
                );
            }
        }
    }

    pub async fn handle_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::Quit => {
                if self.mode == AppMode::Reader {
                    self.save_progress().await;
                }
                if self.reader_view.show_image {
                    self.reader_view.show_image = false;
                    self.reader_view.image_state = None;
                } else if self.reader_view.show_themes {
                    self.reader_view.show_themes = false;
                } else if self.reader_view.show_info {
                    self.reader_view.show_info = false;
                } else if self.reader_view.show_bookmarks {
                    self.reader_view.show_bookmarks = false;
                } else if self.reader_view.show_toc {
                    self.reader_view.show_toc = false;
                } else if self.reader_view.show_help {
                    self.reader_view.show_help = false;
                } else if self.mode == AppMode::Reader {
                    self.end_active_session().await;
                    self.mode = AppMode::Library;
                } else {
                    self.is_running = false;
                }
            }
            KeyAction::OpenFile => {
                self.input_buffer.clear();
                self.mode = AppMode::OpenFileInput;
            }
            KeyAction::Info => {
                if self.mode == AppMode::Reader {
                    self.reader_view.show_info = !self.reader_view.show_info;
                }
            }
            KeyAction::SaveToLibrary => {
                self.save_progress().await;
            }
            KeyAction::Search => {
                if self.mode == AppMode::Reader {
                    self.input_buffer.clear();
                    self.mode = AppMode::SearchInput;
                } else if self.mode == AppMode::Library {
                    self.input_buffer = self.library_view.filter.clone();
                    self.mode = AppMode::LibraryFilterInput;
                }
            }
            KeyAction::Command => {
                self.input_buffer.clear();
                self.mode = AppMode::CommandInput;
            }
            KeyAction::AddBookmark => {
                if self.mode == AppMode::Reader {
                    if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout) {
                        let line_idx = self.reader_view.scroll_offset;
                        let (offset, snippet) = if let Some(line) = layout.lines.get(line_idx) {
                            let text_spans: Vec<String> =
                                line.spans.iter().map(|s| s.text.clone()).collect();
                            let full_line = text_spans.join("").trim().to_string();
                            let preview = if full_line.chars().count() > 50 {
                                let tr: String = full_line.chars().take(47).collect();
                                format!("{}...", tr)
                            } else if full_line.is_empty() {
                                "Empty line".to_string()
                            } else {
                                full_line
                            };
                            (
                                line.char_start,
                                format!("Line {}: \"{}\"", line_idx + 1, preview),
                            )
                        } else {
                            (0, format!("Line {}", line_idx + 1))
                        };

                        let book_id = book.id.clone();
                        let db = self.db.clone();
                        tokio::spawn(async move {
                            let _ = db.add_bookmark(&book_id, offset, &snippet).await;
                        });
                    }
                }
            }
            KeyAction::ListBookmarks => {
                if self.mode == AppMode::Reader {
                    if let Some(book) = &self.active_book {
                        if let Ok(bms) = self.db.list_bookmarks(&book.id).await {
                            self.reader_view.bookmark_items = bms;
                            if !self.reader_view.bookmark_items.is_empty()
                                && self.reader_view.bookmark_state.selected().is_none()
                            {
                                self.reader_view.bookmark_state.select(Some(0));
                            }
                            self.reader_view.show_bookmarks = !self.reader_view.show_bookmarks;
                        }
                    }
                }
            }
            KeyAction::NextMatch => {
                if self.mode == AppMode::Reader && !self.search_matches.is_empty() {
                    self.current_match_idx =
                        (self.current_match_idx + 1) % self.search_matches.len();
                    let m = &self.search_matches[self.current_match_idx];
                    if let Some(layout) = &self.active_layout {
                        self.reader_view.scroll_offset = layout.line_at_char_offset(m.char_start);
                    }
                }
            }
            KeyAction::PrevMatch => {
                if self.mode == AppMode::Reader && !self.search_matches.is_empty() {
                    if self.current_match_idx == 0 {
                        self.current_match_idx = self.search_matches.len() - 1;
                    } else {
                        self.current_match_idx -= 1;
                    }
                    let m = &self.search_matches[self.current_match_idx];
                    if let Some(layout) = &self.active_layout {
                        self.reader_view.scroll_offset = layout.line_at_char_offset(m.char_start);
                    }
                }
            }
            KeyAction::ViewImage => {
                if self.mode == AppMode::Reader {
                    self.handle_view_image().await;
                }
            }
            KeyAction::Delete => {
                if self.mode == AppMode::Reader && self.reader_view.show_bookmarks {
                    let idx = self.reader_view.bookmark_state.selected().unwrap_or(0);
                    let target = self.reader_view.bookmark_items.get(idx).cloned();
                    if let Some(bm) = target {
                        let _ = self.db.delete_bookmark(&bm.id).await;
                        if let Some(book) = &self.active_book {
                            if let Ok(bms) = self.db.list_bookmarks(&book.id).await {
                                self.reader_view.bookmark_items = bms;
                            }
                        }
                        let len = self.reader_view.bookmark_items.len();
                        if len == 0 {
                            self.reader_view.bookmark_state.select(None);
                        } else if idx >= len {
                            self.reader_view.bookmark_state.select(Some(len - 1));
                        }
                        self.status_message = Some(format!("Deleted bookmark '{}'", bm.label));
                    }
                } else if self.mode == AppMode::Library {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    let target = self.visible_library_books().get(idx).map(|b| (*b).clone());
                    if let Some(db_book) = target {
                        let _ = self.db.delete_book(&db_book.id).await;
                        let _ = self.refresh_library().await;
                        let new_len = self.visible_library_books().len();
                        if new_len == 0 {
                            self.library_view.state.select(None);
                        } else if idx >= new_len {
                            self.library_view.state.select(Some(new_len - 1));
                        }
                        self.status_message =
                            Some(format!("Deleted '{}' from library", db_book.title));
                    }
                }
            }
            KeyAction::CycleSort => {
                if self.mode == AppMode::Library {
                    self.library_view.sort_mode = self.library_view.sort_mode.next();
                    self.library_view.state.select(Some(0));
                }
            }
            KeyAction::Select => {
                if self.reader_view.show_themes {
                    let idx = self.reader_view.theme_state.selected().unwrap_or(0);
                    if let Some(&name) = crate::themes::THEME_NAMES.get(idx) {
                        self.config.theme = name.to_string();
                        self.theme = Theme::get_by_name(name);
                    }
                    self.reader_view.show_themes = false;
                } else if self.mode == AppMode::Library && !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    let selected_path = self
                        .visible_library_books()
                        .get(idx)
                        .map(|b| b.file_path.clone());
                    if let Some(file_path) = selected_path {
                        let path = std::path::PathBuf::from(&file_path);
                        match crate::formats::parse_book_file(&path) {
                            Ok(book) => self.load_book(book).await,
                            Err(e) => self.status_message = Some(format!("{}", e)),
                        }
                    }
                } else if self.mode == AppMode::Reader && self.reader_view.show_toc {
                    if let Some(book) = &self.active_book {
                        let idx = self.reader_view.toc_state.selected().unwrap_or(0);
                        if let Some(toc_item) = book.toc.get(idx) {
                            if let Some(layout) = &self.active_layout {
                                self.reader_view.scroll_offset =
                                    layout.line_at_char_offset(toc_item.block_index);
                            }
                        }
                    }
                    self.reader_view.show_toc = false;
                } else if self.mode == AppMode::Reader && self.reader_view.show_bookmarks {
                    let idx = self.reader_view.bookmark_state.selected().unwrap_or(0);
                    if let Some(bm) = self.reader_view.bookmark_items.get(idx) {
                        if let Some(layout) = &self.active_layout {
                            self.reader_view.scroll_offset =
                                layout.line_at_char_offset(bm.char_offset as usize);
                        }
                    }
                    self.reader_view.show_bookmarks = false;
                }
            }
            KeyAction::ScrollDown => {
                if self.reader_view.show_themes {
                    let idx = self.reader_view.theme_state.selected().unwrap_or(0);
                    if idx + 1 < crate::themes::THEME_NAMES.len() {
                        self.reader_view.theme_state.select(Some(idx + 1));
                    }
                } else if self.mode == AppMode::Reader {
                    if self.reader_view.show_bookmarks {
                        let idx = self.reader_view.bookmark_state.selected().unwrap_or(0);
                        if idx + 1 < self.reader_view.bookmark_items.len() {
                            self.reader_view.bookmark_state.select(Some(idx + 1));
                        }
                    } else if self.reader_view.show_toc {
                        if let Some(book) = &self.active_book {
                            let idx = self.reader_view.toc_state.selected().unwrap_or(0);
                            if idx + 1 < book.toc.len() {
                                self.reader_view.toc_state.select(Some(idx + 1));
                            }
                        }
                    } else if let Some(layout) = &self.active_layout {
                        if self.reader_view.scroll_offset + 1 < layout.lines.len() {
                            self.reader_view.scroll_offset += 1;
                        }
                    }
                } else if self.mode == AppMode::Library && !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    let len = self.visible_library_books().len();
                    if idx + 1 < len {
                        self.library_view.state.select(Some(idx + 1));
                    }
                }
            }
            KeyAction::ScrollUp => {
                if self.reader_view.show_themes {
                    let idx = self.reader_view.theme_state.selected().unwrap_or(0);
                    self.reader_view
                        .theme_state
                        .select(Some(idx.saturating_sub(1)));
                } else if self.mode == AppMode::Reader {
                    if self.reader_view.show_bookmarks {
                        let idx = self.reader_view.bookmark_state.selected().unwrap_or(0);
                        self.reader_view
                            .bookmark_state
                            .select(Some(idx.saturating_sub(1)));
                    } else if self.reader_view.show_toc {
                        let idx = self.reader_view.toc_state.selected().unwrap_or(0);
                        self.reader_view
                            .toc_state
                            .select(Some(idx.saturating_sub(1)));
                    } else {
                        self.reader_view.scroll_offset =
                            self.reader_view.scroll_offset.saturating_sub(1);
                    }
                } else if self.mode == AppMode::Library && !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    self.library_view.state.select(Some(idx.saturating_sub(1)));
                }
            }
            KeyAction::PageDown => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
                        let page = self.reader_view.last_viewport_height.max(1);
                        self.reader_view.scroll_offset = (self.reader_view.scroll_offset + page)
                            .min(layout.lines.len().saturating_sub(1));
                        self.session_pages_read = self.session_pages_read.saturating_add(1);
                    }
                }
            }
            KeyAction::PageUp => {
                if self.mode == AppMode::Reader {
                    let page = self.reader_view.last_viewport_height.max(1);
                    self.reader_view.scroll_offset =
                        self.reader_view.scroll_offset.saturating_sub(page);
                    self.session_pages_read = self.session_pages_read.saturating_add(1);
                }
            }
            KeyAction::HalfPageDown => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
                        let half_page = (self.reader_view.last_viewport_height / 2).max(1);
                        self.reader_view.scroll_offset = (self.reader_view.scroll_offset
                            + half_page)
                            .min(layout.lines.len().saturating_sub(1));
                        self.session_pages_read = self.session_pages_read.saturating_add(1);
                    }
                }
            }
            KeyAction::HalfPageUp => {
                if self.mode == AppMode::Reader {
                    let half_page = (self.reader_view.last_viewport_height / 2).max(1);
                    self.reader_view.scroll_offset =
                        self.reader_view.scroll_offset.saturating_sub(half_page);
                    self.session_pages_read = self.session_pages_read.saturating_add(1);
                }
            }
            KeyAction::GotoTop => {
                if self.mode == AppMode::Reader {
                    self.reader_view.scroll_offset = 0;
                }
            }
            KeyAction::GotoBottom => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
                        self.reader_view.scroll_offset = layout.lines.len().saturating_sub(1);
                    }
                }
            }
            KeyAction::Toc => {
                if self.mode == AppMode::Reader {
                    self.reader_view.show_toc = !self.reader_view.show_toc;
                }
            }
            KeyAction::Help => {
                self.reader_view.show_help = !self.reader_view.show_help;
            }
            KeyAction::ToggleSimpleMode => {
                self.config.display.simplified_mode = !self.config.display.simplified_mode;
                self.rebuild_layout_preserving_position();
            }
            KeyAction::ToggleCss => {
                self.config.display.respect_epub_css = !self.config.display.respect_epub_css;
            }
            KeyAction::ToggleJustify => {
                self.config.typography.justified = !self.config.typography.justified;
                self.rebuild_layout_preserving_position();
            }
            KeyAction::ToggleWidescreen => {
                // Widescreen only affects the reader's horizontal centering
                // margin (see `ReaderView::render`), not line wrapping, so
                // no layout rebuild (and thus no position drift) is needed.
                self.config.display.widescreen = !self.config.display.widescreen;
            }
        }
    }

    async fn handle_open_file_input_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Enter => {
                let path_str = self.input_buffer.trim().to_string();
                self.input_buffer.clear();
                match crate::formats::parse_book_uri_async(&path_str).await {
                    Ok(book) => {
                        self.status_message = None;
                        self.load_book(book).await;
                    }
                    Err(e) => {
                        self.status_message = Some(format!("{}", e));
                        self.mode = if self.active_book.is_some() {
                            AppMode::Reader
                        } else {
                            AppMode::Library
                        };
                    }
                }
            }
            KeyCode::Esc => {
                self.input_buffer.clear();
                self.mode = if self.active_book.is_some() {
                    AppMode::Reader
                } else {
                    AppMode::Library
                };
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    async fn handle_search_input_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                if !self.search_history.is_empty() {
                    let new_idx = match self.search_history_idx {
                        None => self.search_history.len().saturating_sub(1),
                        Some(idx) => idx.saturating_sub(1),
                    };
                    self.search_history_idx = Some(new_idx);
                    self.input_buffer = self.search_history[new_idx].clone();
                }
            }
            KeyCode::Down => {
                if let Some(idx) = self.search_history_idx {
                    if idx + 1 < self.search_history.len() {
                        self.search_history_idx = Some(idx + 1);
                        self.input_buffer = self.search_history[idx + 1].clone();
                    } else {
                        self.search_history_idx = None;
                        self.input_buffer.clear();
                    }
                }
            }
            KeyCode::Enter => {
                let query = self.input_buffer.clone();
                if !query.trim().is_empty() && self.search_history.last() != Some(&query) {
                    self.search_history.push(query.clone());
                }
                self.search_history_idx = None;
                self.input_buffer.clear();
                self.mode = AppMode::Reader;
                if let Some(index) = &self.search_index {
                    self.search_matches = index.search(&query);
                    self.current_match_idx = 0;
                    if let Some(m) = self.search_matches.first() {
                        if let Some(layout) = &self.active_layout {
                            self.reader_view.scroll_offset =
                                layout.line_at_char_offset(m.char_start);
                        }
                    }
                }
            }
            KeyCode::Esc => {
                self.input_buffer.clear();
                self.search_history_idx = None;
                self.mode = AppMode::Reader;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    async fn handle_library_filter_input_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Enter => {
                self.mode = AppMode::Library;
            }
            KeyCode::Esc => {
                self.input_buffer.clear();
                self.library_view.filter.clear();
                self.library_view.state.select(Some(0));
                self.mode = AppMode::Library;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.library_view.filter = self.input_buffer.clone();
                self.library_view.state.select(Some(0));
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.library_view.filter = self.input_buffer.clone();
                self.library_view.state.select(Some(0));
            }
            _ => {}
        }
    }

    async fn handle_command_input_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                if !self.command_history.is_empty() {
                    let new_idx = match self.command_history_idx {
                        None => self.command_history.len().saturating_sub(1),
                        Some(idx) => idx.saturating_sub(1),
                    };
                    self.command_history_idx = Some(new_idx);
                    self.input_buffer = self.command_history[new_idx].clone();
                }
            }
            KeyCode::Down => {
                if let Some(idx) = self.command_history_idx {
                    if idx + 1 < self.command_history.len() {
                        self.command_history_idx = Some(idx + 1);
                        self.input_buffer = self.command_history[idx + 1].clone();
                    } else {
                        self.command_history_idx = None;
                        self.input_buffer.clear();
                    }
                }
            }
            KeyCode::Enter => {
                self.execute_command().await;
            }
            KeyCode::Esc => {
                self.input_buffer.clear();
                self.mode = if self.active_book.is_some() {
                    AppMode::Reader
                } else {
                    AppMode::Library
                };
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    /// Parse and run the command currently in `input_buffer` (invoked on
    /// Enter from `CommandInput` mode).
    async fn execute_command(&mut self) {
        let cmd = self.input_buffer.trim().to_string();
        if !cmd.is_empty() && self.command_history.last() != Some(&cmd) {
            self.command_history.push(cmd.clone());
        }
        self.command_history_idx = None;
        self.input_buffer.clear();
        let default_mode = if self.active_book.is_some() {
            AppMode::Reader
        } else {
            AppMode::Library
        };
        self.mode = default_mode;

        if cmd == "q" || cmd == "quit" {
            if self.mode == AppMode::Reader {
                self.mode = AppMode::Library;
            } else {
                self.is_running = false;
            }
        } else if cmd == "qa" || cmd == "quitall" {
            self.is_running = false;
        } else if cmd == "config edit" || cmd == "config" {
            let config_path = self.config_path.clone();
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if !config_path.exists() {
                if let Ok(default_toml) = self.config.serialize_to_toml() {
                    let _ = std::fs::write(&config_path, default_toml);
                }
            }
            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "nano".to_string());

            let _ = disable_raw_mode();
            let _ = stdout().execute(LeaveAlternateScreen);

            let child = std::process::Command::new(editor).arg(&config_path).spawn();
            if let Ok(mut child) = child {
                let _ = child.wait();
            }

            let _ = enable_raw_mode();
            let _ = stdout().execute(EnterAlternateScreen);

            if let Ok(new_config) = crate::config::Config::load_from_file(&config_path) {
                self.config = new_config;
                self.theme = Theme::get_by_name(&self.config.theme);
                self.rebuild_layout_preserving_position();
            }
        } else if cmd == "themes" || cmd == "theme" {
            self.reader_view.show_themes = true;
        } else if let Some(theme_name) = cmd.strip_prefix("theme ") {
            self.config.theme = theme_name.trim().to_string();
            self.theme = Theme::get_by_name(theme_name.trim());
        } else if let Some(path_str) = cmd.strip_prefix("open ").or_else(|| cmd.strip_prefix("o "))
        {
            match crate::formats::parse_book_uri_async(path_str.trim()).await {
                Ok(book) => {
                    self.status_message = None;
                    self.load_book(book).await;
                }
                Err(e) => {
                    self.status_message = Some(format!("{}", e));
                }
            }
        } else if let Some(dir_str) = cmd.strip_prefix("scan ") {
            let dir = std::path::PathBuf::from(dir_str.trim());
            let summary = crate::library::scan_and_import(&self.db, &dir).await;
            let _ = self.refresh_library().await;
            self.status_message = Some(format!(
                "Scanned '{}': {} imported, {} already known, {} failed",
                dir.display(),
                summary.imported,
                summary.skipped,
                summary.failed
            ));
        } else if cmd == "w" || cmd == "save" {
            self.handle_action(KeyAction::SaveToLibrary).await;
        } else if cmd == "b" || cmd == "bookmark" {
            self.handle_action(KeyAction::AddBookmark).await;
        } else if cmd == "bl" || cmd == "bookmarks" {
            self.handle_action(KeyAction::ListBookmarks).await;
        } else if cmd == "toc" || cmd == "t" {
            self.reader_view.show_toc = true;
        } else if cmd == "info" || cmd == "i" {
            self.reader_view.show_info = true;
        } else if cmd == "simplified" || cmd == "simple" {
            self.handle_action(KeyAction::ToggleSimpleMode).await;
        } else if cmd == "opds" || cmd == "opds open" {
            let default_url = "https://www.gutenberg.org/ebooks/search.opds/".to_string();
            if let Err(e) = self.open_opds_url(&default_url).await {
                self.status_message = Some(format!("{}", e));
            }
        } else if let Some(rest) = cmd.strip_prefix("opds add ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let url = parts[1].to_string();
                self.opds_catalogs.insert(name.clone(), url.clone());
                self.config.opds_catalogs.insert(name.clone(), url);
                match self.save_config() {
                    Ok(()) => {
                        self.status_message =
                            Some(format!("Added and saved OPDS catalog '{}'", name));
                    }
                    Err(e) => {
                        self.status_message = Some(format!(
                            "Added OPDS catalog '{}' but failed to save config: {}",
                            name, e
                        ));
                    }
                }
            } else {
                self.status_message = Some("Usage: :opds add <name> <url>".to_string());
            }
        } else if let Some(rest) = cmd
            .strip_prefix("opds open ")
            .or_else(|| cmd.strip_prefix("opds "))
        {
            let target = rest.trim();
            let url = self
                .opds_catalogs
                .get(target)
                .cloned()
                .unwrap_or_else(|| target.to_string());
            if let Err(e) = self.open_opds_url(&url).await {
                self.status_message = Some(format!("{}", e));
            }
        } else if cmd == "help" || cmd == "h" {
            self.reader_view.show_help = true;
        } else if cmd == "widescreen" || cmd == "wide" {
            self.handle_action(KeyAction::ToggleWidescreen).await;
        } else if cmd == "css" {
            self.handle_action(KeyAction::ToggleCss).await;
        } else if let Ok(line_num) = cmd.parse::<usize>() {
            if line_num > 0 {
                self.reader_view.scroll_offset = line_num.saturating_sub(1);
            }
        } else if let Some(num_str) = cmd.strip_prefix("goto ") {
            if let Ok(line_num) = num_str.trim().parse::<usize>() {
                if line_num > 0 {
                    self.reader_view.scroll_offset = line_num.saturating_sub(1);
                }
            }
        }
    }

    async fn handle_opds_browser_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(feed) = self.opds_feed_stack.last() {
                    let idx = self.opds_view.state.selected().unwrap_or(0);
                    if idx + 1 < feed.entries.len() {
                        self.opds_view.state.select(Some(idx + 1));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let idx = self.opds_view.state.selected().unwrap_or(0);
                self.opds_view.state.select(Some(idx.saturating_sub(1)));
            }
            KeyCode::Enter => {
                if let Some(feed) = self.opds_feed_stack.last().cloned() {
                    let idx = self.opds_view.state.selected().unwrap_or(0);
                    if let Some(entry) = feed.entries.get(idx) {
                        match &entry.link {
                            crate::opds::OpdsLinkType::Catalog(url) => {
                                if let Err(e) = self.open_opds_url(url).await {
                                    self.status_message = Some(format!("{}", e));
                                }
                            }
                            crate::opds::OpdsLinkType::Acquisition(url) => {
                                match crate::formats::parse_book_uri_async(url).await {
                                    Ok(book) => {
                                        let _ = self.db.upsert_book(&book, 0, 0.0).await;
                                        self.load_book(book).await;
                                    }
                                    Err(e) => {
                                        self.status_message =
                                            Some(format!("Download error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                self.input_buffer.clear();
                self.mode = AppMode::SearchInput;
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.opds_feed_stack.pop();
                if self.opds_feed_stack.is_empty() {
                    self.mode = if self.active_book.is_some() {
                        AppMode::Reader
                    } else {
                        AppMode::Library
                    };
                } else {
                    self.opds_view.state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    /// Detect (or apply the configured override for) the terminal's image
    /// graphics protocol. Must be called after entering the alternate
    /// screen but before reading terminal events.
    fn init_image_picker(&mut self) {
        if self.config.display.image_protocol == "none" {
            self.image_picker = None;
            return;
        }

        self.image_picker = match Picker::from_query_stdio() {
            Ok(mut picker) => {
                match self.config.display.image_protocol.as_str() {
                    "kitty" => picker.set_protocol_type(ProtocolType::Kitty),
                    "iterm2" => picker.set_protocol_type(ProtocolType::Iterm2),
                    "sixel" => picker.set_protocol_type(ProtocolType::Sixel),
                    "halfblocks" => picker.set_protocol_type(ProtocolType::Halfblocks),
                    // "auto" or unrecognized: keep the detected protocol.
                    _ => {}
                }
                Some(picker)
            }
            Err(_) => None,
        };
    }

    pub async fn run_tui(&mut self) -> Result<()> {
        install_panic_hook();

        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;

        self.init_image_picker();

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;

        self.refresh_library().await?;

        let mut event_stream = EventStream::new();

        while self.is_running {
            terminal.draw(|f| {
                let area = f.area();
                let status_msg = self.status_message.as_deref();
                match self.mode {
                    AppMode::Library => {
                        self.library_view.render(
                            f,
                            area,
                            &self.library_books,
                            &self.theme,
                            status_msg,
                        );
                    }
                    AppMode::Reader => {
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(
                                f,
                                area,
                                book,
                                layout,
                                &self.config,
                                &self.theme,
                                status_msg,
                            );
                        }
                    }
                    AppMode::SearchInput => {
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(
                                f,
                                area,
                                book,
                                layout,
                                &self.config,
                                &self.theme,
                                status_msg,
                            );
                            let chunks = ratatui::layout::Layout::default()
                                .direction(ratatui::layout::Direction::Vertical)
                                .constraints([
                                    ratatui::layout::Constraint::Min(1),
                                    ratatui::layout::Constraint::Length(1),
                                ])
                                .split(area);
                            f.render_widget(ratatui::widgets::Clear, chunks[1]);
                            let prompt =
                                ratatui::widgets::Paragraph::new(format!("/{}", self.input_buffer))
                                    .style(self.theme.status_style());
                            f.render_widget(prompt, chunks[1]);
                            f.set_cursor_position((
                                chunks[1].x + 1 + self.input_buffer.len() as u16,
                                chunks[1].y,
                            ));
                        }
                    }
                    AppMode::CommandInput => {
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(
                                f,
                                area,
                                book,
                                layout,
                                &self.config,
                                &self.theme,
                                status_msg,
                            );
                        } else {
                            self.library_view.render(
                                f,
                                area,
                                &self.library_books,
                                &self.theme,
                                status_msg,
                            );
                        }
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([
                                ratatui::layout::Constraint::Min(1),
                                ratatui::layout::Constraint::Length(1),
                            ])
                            .split(area);
                        f.render_widget(ratatui::widgets::Clear, chunks[1]);
                        let prompt =
                            ratatui::widgets::Paragraph::new(format!(":{}", self.input_buffer))
                                .style(self.theme.status_style());
                        f.render_widget(prompt, chunks[1]);
                        f.set_cursor_position((
                            chunks[1].x + 1 + self.input_buffer.len() as u16,
                            chunks[1].y,
                        ));
                    }
                    AppMode::OpenFileInput => {
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([
                                ratatui::layout::Constraint::Min(1),
                                ratatui::layout::Constraint::Length(1),
                            ])
                            .split(area);
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(
                                f,
                                area,
                                book,
                                layout,
                                &self.config,
                                &self.theme,
                                status_msg,
                            );
                        } else {
                            self.library_view.render(
                                f,
                                area,
                                &self.library_books,
                                &self.theme,
                                status_msg,
                            );
                        }
                        f.render_widget(ratatui::widgets::Clear, chunks[1]);
                        let prompt = ratatui::widgets::Paragraph::new(format!(
                            "Open File: {}",
                            self.input_buffer
                        ))
                        .style(self.theme.status_style());
                        f.render_widget(prompt, chunks[1]);
                        f.set_cursor_position((
                            chunks[1].x + 11 + self.input_buffer.len() as u16,
                            chunks[1].y,
                        ));
                    }
                    AppMode::LibraryFilterInput => {
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([
                                ratatui::layout::Constraint::Min(1),
                                ratatui::layout::Constraint::Length(1),
                            ])
                            .split(area);
                        self.library_view.render(
                            f,
                            chunks[0],
                            &self.library_books,
                            &self.theme,
                            status_msg,
                        );
                        f.render_widget(ratatui::widgets::Clear, chunks[1]);
                        let prompt = ratatui::widgets::Paragraph::new(format!(
                            "Filter: {}",
                            self.input_buffer
                        ))
                        .style(self.theme.status_style());
                        f.render_widget(prompt, chunks[1]);
                        f.set_cursor_position((
                            chunks[1].x + 8 + self.input_buffer.len() as u16,
                            chunks[1].y,
                        ));
                    }
                    AppMode::OpdsBrowser => {
                        if let Some(feed) = self.opds_feed_stack.last() {
                            self.opds_view.render(f, area, feed, &self.theme);
                        }
                    }
                }
            })?;

            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    if let Event::Key(key_event) = event {
                        self.status_message = None;

                        // Support Ctrl+V paste in input modes
                        if matches!(
                            self.mode,
                            AppMode::SearchInput
                                | AppMode::CommandInput
                                | AppMode::OpenFileInput
                                | AppMode::LibraryFilterInput
                        ) && key_event.modifiers.contains(KeyModifiers::CONTROL)
                            && (key_event.code == KeyCode::Char('v')
                                || key_event.code == KeyCode::Char('V'))
                        {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    let clean_text = text.replace('\r', "").replace('\n', " ");
                                    self.input_buffer.push_str(&clean_text);
                                    if self.mode == AppMode::LibraryFilterInput {
                                        self.library_view.filter = self.input_buffer.clone();
                                    }
                                }
                            }
                            continue;
                        }

                        match self.mode {
                            AppMode::OpenFileInput => self.handle_open_file_input_key(key_event).await,
                            AppMode::SearchInput => self.handle_search_input_key(key_event).await,
                            AppMode::CommandInput => self.handle_command_input_key(key_event).await,
                            AppMode::LibraryFilterInput => {
                                self.handle_library_filter_input_key(key_event).await
                            }
                            AppMode::OpdsBrowser => self.handle_opds_browser_key(key_event).await,
                            _ => {
                                if let Some(action) = self.key_dispatcher.handle_event(key_event, &self.config.keymap) {
                                    self.handle_action(action).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;

        Ok(())
    }
}

/// Ensure the terminal is restored to a sane state (raw mode disabled,
/// alternate screen exited) even if the app panics mid-render, instead of
/// leaving the user's terminal unusable until they run `reset`.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
