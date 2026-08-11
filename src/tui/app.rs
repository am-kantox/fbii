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
use crossterm::event::{Event, EventStream};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Library,
    Reader,
    SearchInput,
    CommandInput,
}

pub struct App {
    pub mode: AppMode,
    pub config: Config,
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
    pub is_running: bool,
}

impl App {
    pub fn new(config: Config, db: LibraryDb) -> Self {
        let theme = Theme::get_by_name(&config.theme);
        Self {
            mode: AppMode::Library,
            config,
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
            input_buffer: String::new(),
            bookmarks: Vec::new(),
            is_running: true,
        }
    }

    pub fn load_book(&mut self, book: Book) {
        let layout = BookLayout::build(
            &book,
            &self.config.typography,
            self.config.display.simplified_mode,
        );
        let search_index = BookSearchIndex::build(&book);

        self.active_book = Some(book);
        self.active_layout = Some(layout);
        self.search_index = Some(search_index);
        self.reader_view.scroll_offset = 0;
        self.mode = AppMode::Reader;
    }

    pub async fn refresh_library(&mut self) -> Result<()> {
        self.library_books = self.db.list_books().await?;
        Ok(())
    }

    pub fn handle_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::Quit => {
                if self.reader_view.show_bookmarks {
                    self.reader_view.show_bookmarks = false;
                } else if self.reader_view.show_toc {
                    self.reader_view.show_toc = false;
                } else if self.reader_view.show_help {
                    self.reader_view.show_help = false;
                } else if self.mode == AppMode::Reader {
                    self.mode = AppMode::Library;
                } else {
                    self.is_running = false;
                }
            }
            KeyAction::Search => {
                if self.mode == AppMode::Reader {
                    self.input_buffer.clear();
                    self.mode = AppMode::SearchInput;
                }
            }
            KeyAction::Command => {
                self.input_buffer.clear();
                self.mode = AppMode::CommandInput;
            }
            KeyAction::AddBookmark => {
                if self.mode == AppMode::Reader {
                    if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout) {
                        let offset = layout
                            .lines
                            .get(self.reader_view.scroll_offset)
                            .map(|l| l.char_start)
                            .unwrap_or(0);
                        let label = format!("Line {}", self.reader_view.scroll_offset + 1);
                        let book_id = book.id.clone();
                        let db = self.db.clone();
                        tokio::spawn(async move {
                            let _ = db.add_bookmark(&book_id, offset, &label).await;
                        });
                    }
                }
            }
            KeyAction::ListBookmarks => {
                if self.mode == AppMode::Reader {
                    if let Some(book) = &self.active_book {
                        let book_id = book.id.clone();
                        let db = self.db.clone();
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        tokio::spawn(async move {
                            if let Ok(bms) = db.list_bookmarks(&book_id).await {
                                let _ = tx.send(bms);
                            }
                        });
                        if let Ok(bms) = rx.blocking_recv() {
                            self.reader_view.bookmark_items = bms;
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
            KeyAction::Select => {
                if self.mode == AppMode::Library && !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    if let Some(db_book) = self.library_books.get(idx) {
                        let path = std::path::PathBuf::from(&db_book.file_path);
                        if let Ok(book) = crate::formats::parse_book_file(&path) {
                            self.load_book(book);
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
                if self.mode == AppMode::Reader {
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
                } else if !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    if idx + 1 < self.library_books.len() {
                        self.library_view.state.select(Some(idx + 1));
                    }
                }
            }
            KeyAction::ScrollUp => {
                if self.mode == AppMode::Reader {
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
                } else if !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    self.library_view.state.select(Some(idx.saturating_sub(1)));
                }
            }
            KeyAction::PageDown => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
                        self.reader_view.scroll_offset = (self.reader_view.scroll_offset + 25)
                            .min(layout.lines.len().saturating_sub(1));
                    }
                }
            }
            KeyAction::PageUp => {
                if self.mode == AppMode::Reader {
                    self.reader_view.scroll_offset =
                        self.reader_view.scroll_offset.saturating_sub(25);
                }
            }
            KeyAction::HalfPageDown => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
                        self.reader_view.scroll_offset = (self.reader_view.scroll_offset + 10)
                            .min(layout.lines.len().saturating_sub(1));
                    }
                }
            }
            KeyAction::HalfPageUp => {
                if self.mode == AppMode::Reader {
                    self.reader_view.scroll_offset =
                        self.reader_view.scroll_offset.saturating_sub(10);
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
                if let Some(book) = &self.active_book {
                    let layout = BookLayout::build(
                        book,
                        &self.config.typography,
                        self.config.display.simplified_mode,
                    );
                    self.active_layout = Some(layout);
                }
            }
            _ => {}
        }
    }

    pub async fn run_tui(&mut self) -> Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout());
        let mut terminal = Terminal::new(backend)?;

        self.refresh_library().await?;

        let mut event_stream = EventStream::new();

        while self.is_running {
            terminal.draw(|f| {
                let area = f.area();
                match self.mode {
                    AppMode::Library => {
                        self.library_view
                            .render(f, area, &self.library_books, &self.theme);
                    }
                    AppMode::Reader => {
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(f, area, book, layout, &self.theme);
                        }
                    }
                    AppMode::SearchInput => {
                        if let (Some(book), Some(layout)) = (&self.active_book, &self.active_layout)
                        {
                            self.reader_view.render(f, area, book, layout, &self.theme);
                            let chunks = ratatui::layout::Layout::default()
                                .direction(ratatui::layout::Direction::Vertical)
                                .constraints([
                                    ratatui::layout::Constraint::Min(1),
                                    ratatui::layout::Constraint::Length(1),
                                ])
                                .split(area);
                            let prompt =
                                ratatui::widgets::Paragraph::new(format!("/{}", self.input_buffer))
                                    .style(self.theme.status_style());
                            f.render_widget(prompt, chunks[1]);
                        }
                    }
                    AppMode::CommandInput => {
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([
                                ratatui::layout::Constraint::Min(1),
                                ratatui::layout::Constraint::Length(1),
                            ])
                            .split(area);
                        let prompt =
                            ratatui::widgets::Paragraph::new(format!(":{}", self.input_buffer))
                                .style(self.theme.status_style());
                        f.render_widget(prompt, chunks[1]);
                    }
                }
            })?;

            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    if let Event::Key(key_event) = event {
                        match self.mode {
                            AppMode::SearchInput => {
                                match key_event.code {
                                    crossterm::event::KeyCode::Enter => {
                                        let query = self.input_buffer.clone();
                                        self.mode = AppMode::Reader;
                                        if let Some(index) = &self.search_index {
                                            self.search_matches = index.search(&query);
                                            self.current_match_idx = 0;
                                            if let Some(m) = self.search_matches.first() {
                                                if let Some(layout) = &self.active_layout {
                                                    self.reader_view.scroll_offset = layout.line_at_char_offset(m.char_start);
                                                }
                                            }
                                        }
                                    }
                                    crossterm::event::KeyCode::Esc => {
                                        self.input_buffer.clear();
                                        self.mode = AppMode::Reader;
                                    }
                                    crossterm::event::KeyCode::Backspace => {
                                        self.input_buffer.pop();
                                    }
                                    crossterm::event::KeyCode::Char(c) => {
                                        self.input_buffer.push(c);
                                    }
                                    _ => {}
                                }
                            }
                            AppMode::CommandInput => {
                                match key_event.code {
                                    crossterm::event::KeyCode::Enter => {
                                        let cmd = self.input_buffer.trim().to_string();
                                        self.input_buffer.clear();
                                        self.mode = AppMode::Reader;
                                        if cmd == "q" || cmd == "quit" {
                                            self.is_running = false;
                                        } else if cmd == "toc" {
                                            self.reader_view.show_toc = true;
                                        } else if cmd == "help" {
                                            self.reader_view.show_help = true;
                                        }
                                    }
                                    crossterm::event::KeyCode::Esc => {
                                        self.input_buffer.clear();
                                        self.mode = AppMode::Reader;
                                    }
                                    crossterm::event::KeyCode::Backspace => {
                                        self.input_buffer.pop();
                                    }
                                    crossterm::event::KeyCode::Char(c) => {
                                        self.input_buffer.push(c);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                if let Some(action) = self.key_dispatcher.handle_event(key_event, &self.config.keymap) {
                                    self.handle_action(action);
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
