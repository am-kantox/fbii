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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Library,
    Reader,
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
                if self.reader_view.show_toc {
                    self.reader_view.show_toc = false;
                } else if self.reader_view.show_help {
                    self.reader_view.show_help = false;
                } else if self.mode == AppMode::Reader {
                    self.mode = AppMode::Library;
                } else {
                    self.is_running = false;
                }
            }
            KeyAction::ScrollDown => {
                if self.mode == AppMode::Reader {
                    if let Some(layout) = &self.active_layout {
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
                    self.reader_view.scroll_offset =
                        self.reader_view.scroll_offset.saturating_sub(1);
                } else if !self.library_books.is_empty() {
                    let idx = self.library_view.state.selected().unwrap_or(0);
                    self.library_view.state.select(Some(idx.saturating_sub(1)));
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
                }
            })?;

            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    if let Event::Key(key_event) = event {
                        if let Some(action) = self.key_dispatcher.handle_event(key_event, &self.config.keymap) {
                            self.handle_action(action);
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
