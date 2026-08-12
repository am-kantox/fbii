use crate::opds::{OpdsFeed, OpdsLinkType};
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Default)]
pub struct OpdsView {
    pub state: ListState,
}

impl OpdsView {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, feed: &OpdsFeed, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        let title_text = format!(" OPDS Catalog: {} ", feed.title);

        if feed.entries.is_empty() {
            let empty_msg =
                Paragraph::new("No items in this OPDS feed. Press 'Esc' or 'q' to go back.")
                    .style(theme.base_style())
                    .block(Block::default().borders(Borders::ALL).title(title_text));
            f.render_widget(empty_msg, chunks[0]);
        } else {
            let items: Vec<ListItem> = feed
                .entries
                .iter()
                .map(|e| {
                    let badge = match e.link {
                        OpdsLinkType::Catalog(_) => "[Catalog]",
                        OpdsLinkType::Acquisition(_) => "[Book]",
                    };
                    let text = format!("{} {} - {}", badge, e.title, e.author);
                    ListItem::new(text).style(theme.base_style())
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title_text))
                .highlight_style(
                    Style::default()
                        .bg(theme.selection)
                        .fg(theme.foreground)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[0], &mut self.state);
        }

        let help = Paragraph::new(
            "[j/k] Navigate | [Enter] Open Catalog / Download Book | [/] Search | [q/Esc] Back",
        )
        .style(theme.status_style());
        f.render_widget(help, chunks[1]);
    }
}
