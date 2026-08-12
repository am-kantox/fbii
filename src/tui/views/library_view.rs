use crate::db::DbBook;
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Default)]
pub struct LibraryView {
    pub state: ListState,
}

impl LibraryView {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
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

        if books.is_empty() {
            let empty_msg = Paragraph::new("Library is empty. Press 'o' to open an e-book file.")
                .style(theme.base_style())
                .block(Block::default().borders(Borders::ALL).title(" Library "));
            f.render_widget(empty_msg, chunks[0]);
        } else {
            let items: Vec<ListItem> = books
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
                .block(Block::default().borders(Borders::ALL).title(" Library "))
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
            "[j/k] Navigate | [Enter] Read | [o] Open File | [q] Quit".to_string()
        };

        let help = Paragraph::new(footer_text).style(theme.status_style());
        f.render_widget(help, chunks[1]);
    }
}
