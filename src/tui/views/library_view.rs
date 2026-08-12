use crate::db::DbBook;
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

/// How the library list is ordered. Cycled with the `CycleSort` action.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LibrarySortMode {
    /// Preserves the database's natural order (most recently updated first).
    #[default]
    RecentlyUpdated,
    Title,
    Author,
}

impl LibrarySortMode {
    pub fn next(self) -> Self {
        match self {
            LibrarySortMode::RecentlyUpdated => LibrarySortMode::Title,
            LibrarySortMode::Title => LibrarySortMode::Author,
            LibrarySortMode::Author => LibrarySortMode::RecentlyUpdated,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LibrarySortMode::RecentlyUpdated => "Recent",
            LibrarySortMode::Title => "Title",
            LibrarySortMode::Author => "Author",
        }
    }
}

#[derive(Default)]
pub struct LibraryView {
    pub state: ListState,
    /// Live text filter (case-insensitive substring match against title and
    /// author), edited via `AppMode::LibraryFilterInput`.
    pub filter: String,
    pub sort_mode: LibrarySortMode,
}

impl LibraryView {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            filter: String::new(),
            sort_mode: LibrarySortMode::default(),
        }
    }

    /// Returns the subset of `books` that match the current filter, ordered
    /// according to the current sort mode. Selection indices in `state`
    /// always refer to positions in this filtered/sorted view, not `books`
    /// directly.
    pub fn visible_books<'a>(&self, books: &'a [DbBook]) -> Vec<&'a DbBook> {
        let filter_lower = self.filter.to_lowercase();
        let mut visible: Vec<&DbBook> = books
            .iter()
            .filter(|b| {
                filter_lower.is_empty()
                    || b.title.to_lowercase().contains(&filter_lower)
                    || b.authors.to_lowercase().contains(&filter_lower)
            })
            .collect();

        match self.sort_mode {
            LibrarySortMode::RecentlyUpdated => {}
            LibrarySortMode::Title => visible.sort_by_key(|a| a.title.to_lowercase()),
            LibrarySortMode::Author => visible.sort_by_key(|a| a.authors.to_lowercase()),
        }

        visible
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        books: &[DbBook],
        theme: &Theme,
        status_message: Option<&str>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        let visible = self.visible_books(books);
        let title = if self.filter.is_empty() {
            format!(" Library ({}) ", self.sort_mode.label())
        } else {
            format!(
                " Library ({}) | filter: {} ",
                self.sort_mode.label(),
                self.filter
            )
        };

        if books.is_empty() {
            let empty_msg = Paragraph::new("Library is empty. Press 'o' to open an e-book file.")
                .style(theme.base_style())
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(empty_msg, chunks[0]);
        } else if visible.is_empty() {
            let empty_msg = Paragraph::new("No books match the current filter.")
                .style(theme.base_style())
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(empty_msg, chunks[0]);
        } else {
            let items: Vec<ListItem> = visible
                .iter()
                .map(|b| {
                    let text = format!(
                        "[{}] {} - {} ({:.1}%)",
                        b.format.to_uppercase(),
                        b.title,
                        b.authors,
                        b.progress_percent
                    );
                    ListItem::new(text).style(theme.base_style())
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(
                    Style::default()
                        .bg(theme.selection)
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[0], &mut self.state);
        }

        let footer_text = if let Some(msg) = status_message {
            format!(" ⚠️ {} ", msg)
        } else {
            "[j/k] Navigate | [Enter] Read | [o] Open | [d] Delete | [r] Sort | [/] Filter | [q] Quit"
                .to_string()
        };

        let help = Paragraph::new(footer_text).style(theme.status_style());
        f.render_widget(help, chunks[1]);
    }
}
