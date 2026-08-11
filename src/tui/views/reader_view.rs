use crate::formats::model::Book;
use crate::renderer::BookLayout;
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

#[derive(Default)]
pub struct ReaderView {
    pub scroll_offset: usize,
    pub show_toc: bool,
    pub show_bookmarks: bool,
    pub show_help: bool,
    pub show_info: bool,
    pub show_themes: bool,
    pub toc_state: ListState,
    pub bookmark_state: ListState,
    pub theme_state: ListState,
    pub bookmark_items: Vec<crate::db::DbBookmark>,
}

impl ReaderView {
    pub fn new() -> Self {
        let mut toc_state = ListState::default();
        toc_state.select(Some(0));
        let mut bookmark_state = ListState::default();
        bookmark_state.select(Some(0));
        let mut theme_state = ListState::default();
        theme_state.select(Some(0));

        Self {
            scroll_offset: 0,
            show_toc: false,
            show_bookmarks: false,
            show_help: false,
            show_info: false,
            show_themes: false,
            toc_state,
            bookmark_state,
            theme_state,
            bookmark_items: Vec::new(),
        }
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        book: &Book,
        layout: &BookLayout,
        theme: &Theme,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);

        let viewport_height = chunks[0].height as usize;
        let visible_lines = layout
            .lines
            .iter()
            .skip(self.scroll_offset)
            .take(viewport_height);

        let mut paragraph_lines = Vec::new();
        for wrapped_line in visible_lines {
            let mut spans = Vec::new();
            for styled in &wrapped_line.spans {
                let mut style = Style::default().fg(theme.foreground);
                if styled.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if styled.italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                if styled.underline {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if styled.strike {
                    style = style.add_modifier(Modifier::CROSSED_OUT);
                }
                if styled.is_heading {
                    style = style.fg(theme.heading).add_modifier(Modifier::BOLD);
                }
                if styled.code {
                    style = style.bg(theme.selection);
                }
                spans.push(Span::styled(styled.text.clone(), style));
            }
            paragraph_lines.push(Line::from(spans));
        }

        let reader_widget = Paragraph::new(paragraph_lines).style(theme.base_style());
        f.render_widget(reader_widget, chunks[0]);

        // Calculate progress %
        let total_lines = layout.lines.len();
        let progress_percent = if total_lines == 0 {
            0.0
        } else {
            ((self.scroll_offset + viewport_height).min(total_lines) as f64 / total_lines as f64)
                * 100.0
        };

        let status_text = format!(
            " {} - {} | Line {}/{} ({:.1}%) | [?] Help ",
            book.metadata.title,
            book.metadata.authors.join(", "),
            self.scroll_offset + 1,
            total_lines.max(1),
            progress_percent
        );

        let status_bar = Paragraph::new(status_text).style(theme.status_style());
        f.render_widget(status_bar, chunks[1]);

        // Render TOC modal if active
        if self.show_toc {
            self.render_toc_modal(f, area, book, theme);
        }

        // Render Bookmarks modal if active
        if self.show_bookmarks {
            self.render_bookmarks_modal(f, area, theme);
        }

        // Render Themes modal if active
        if self.show_themes {
            self.render_themes_modal(f, area, theme);
        }

        // Render Info modal if active
        if self.show_info {
            self.render_info_modal(f, area, book, theme);
        }

        // Render Help modal if active
        if self.show_help {
            self.render_help_modal(f, area, theme);
        }
    }

    pub fn render_bookmarks_modal(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let modal_area = centered_rect(60, 60, area);
        f.render_widget(Clear, modal_area);

        let items: Vec<ListItem> = self
            .bookmark_items
            .iter()
            .map(|b| {
                ListItem::new(format!("Offset {}: {}", b.char_offset, b.label))
                    .style(theme.base_style())
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Bookmarks "))
            .highlight_style(
                Style::default()
                    .bg(theme.selection)
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, modal_area, &mut self.bookmark_state);
    }

    fn render_toc_modal(&mut self, f: &mut Frame, area: Rect, book: &Book, theme: &Theme) {
        let modal_area = centered_rect(60, 60, area);
        f.render_widget(Clear, modal_area);

        let items: Vec<ListItem> = book
            .toc
            .iter()
            .map(|t| ListItem::new(t.title.clone()).style(theme.base_style()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Table of Contents "),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selection)
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, modal_area, &mut self.toc_state);
    }

    fn render_help_modal(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let modal_area = centered_rect(60, 60, area);
        f.render_widget(Clear, modal_area);

        let help_text = r#" Keybindings:
 j / Down         Scroll down 1 line
 k / Up           Scroll up 1 line
 Ctrl+D           Scroll down 1/2 page
 Ctrl+U           Scroll up 1/2 page
 gg               Go to top
 G                Go to bottom
 /                Search query
 n / N            Next / Previous match
 t                Table of Contents
 b                Add Bookmark
 B                List Bookmarks
 S                Toggle Simplified Mode
 q                Back / Quit
"#;

        let paragraph = Paragraph::new(help_text).style(theme.base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Controls & Help "),
        );

        f.render_widget(paragraph, modal_area);
    }

    fn render_themes_modal(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let modal_area = centered_rect(50, 50, area);
        f.render_widget(Clear, modal_area);

        let items: Vec<ListItem> = crate::themes::THEME_NAMES
            .iter()
            .map(|t| ListItem::new(*t).style(theme.base_style()))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Select Theme "),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selection)
                    .fg(theme.foreground)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, modal_area, &mut self.theme_state);
    }

    fn render_info_modal(&mut self, f: &mut Frame, area: Rect, book: &Book, theme: &Theme) {
        let modal_area = centered_rect(60, 60, area);
        f.render_widget(Clear, modal_area);

        let info_text = format!(
            " Title: {}\n Authors: {}\n Format: {}\n Series: {}\n Genres: {}\n Annotation: {}",
            book.metadata.title,
            book.metadata.authors.join(", "),
            book.metadata.format,
            book.metadata
                .series_name
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "N/A".to_string()),
            book.metadata.genres.join(", "),
            book.metadata
                .annotation
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "None".to_string())
        );

        let paragraph = Paragraph::new(info_text).style(theme.base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Book Information "),
        );

        f.render_widget(paragraph, modal_area);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
