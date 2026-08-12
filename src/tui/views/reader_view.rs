use crate::formats::model::Book;
use crate::renderer::BookLayout;
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;

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
    /// Height (in terminal rows) of the last-rendered text viewport. Used to
    /// make page/half-page scrolling match what is actually on screen, and
    /// to compute reading progress consistently with `App::save_progress`.
    pub last_viewport_height: usize,
    /// Whether the inline image viewer modal is currently shown.
    pub show_image: bool,
    /// Resource key of the image currently displayed in the image modal.
    pub current_image_key: Option<String>,
    /// Decoded/resized image render state for the currently viewed image.
    /// Not `Clone`/`Debug`, so it is kept out of any derived traits on
    /// `ReaderView`.
    pub image_state: Option<StatefulProtocol>,
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
            last_viewport_height: 20,
            show_image: false,
            current_image_key: None,
            image_state: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        book: &Book,
        layout: &BookLayout,
        config: &crate::config::Config,
        theme: &Theme,
        status_message: Option<&str>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);

        f.render_widget(ratatui::widgets::Clear, chunks[0]);
        f.render_widget(Block::default().style(theme.base_style()), chunks[0]);

        let viewport_height = chunks[0].height as usize;
        self.last_viewport_height = viewport_height;
        let visible_lines = layout
            .lines
            .iter()
            .skip(self.scroll_offset)
            .take(viewport_height);

        let mut paragraph_lines = Vec::new();
        for line in visible_lines {
            if line.is_empty_line {
                paragraph_lines.push(Line::from(""));
                continue;
            }

            let mut spans = Vec::new();
            for styled in &line.spans {
                let mut style = theme.base_style();
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

        while paragraph_lines.len() < viewport_height {
            paragraph_lines.push(Line::from(""));
        }

        let reader_area = if config.display.widescreen {
            chunks[0]
        } else {
            let content_width = (config.typography.measure).min(chunks[0].width);
            let horizontal_margin = chunks[0].width.saturating_sub(content_width) / 2;
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(horizontal_margin),
                    Constraint::Length(content_width),
                    Constraint::Min(0),
                ])
                .split(chunks[0])[1]
        };

        let reader_widget = Paragraph::new(paragraph_lines).style(theme.base_style());
        f.render_widget(reader_widget, reader_area);

        // Calculate progress % and progress bar
        let total_lines = layout.lines.len();
        let progress_percent = layout.progress_percent(self.scroll_offset, viewport_height);

        let bar_width = 12;
        let filled_len = ((progress_percent / 100.0) * bar_width as f64).round() as usize;
        let filled_len = filled_len.min(bar_width);
        let progress_bar = format!(
            "[{}{}]",
            "█".repeat(filled_len),
            "░".repeat(bar_width - filled_len)
        );

        let status_text = if let Some(msg) = status_message {
            format!(" ⚠️ {} ", msg)
        } else {
            format!(
                " {} - {} | {} Line {}/{} ({:.1}%) | [?] Help ",
                book.metadata.title,
                book.metadata.authors.join(", "),
                progress_bar,
                self.scroll_offset + 1,
                total_lines.max(1),
                progress_percent
            )
        };

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

        // Render the image viewer modal if active
        if self.show_image {
            self.render_image_modal(f, area, theme);
        }
    }

    /// Render the currently-loaded image (if any) in a centered modal using
    /// whatever terminal graphics protocol was detected/configured. Falls
    /// back to a placeholder message when no image data is available.
    fn render_image_modal(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let modal_area = centered_rect(80, 80, area);
        f.render_widget(Clear, modal_area);

        let title = self
            .current_image_key
            .as_deref()
            .unwrap_or("Image")
            .to_string();
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} (press q/Esc to close) ", title));
        let inner = block.inner(modal_area);
        f.render_widget(block, modal_area);

        if let Some(protocol) = self.image_state.as_mut() {
            let image_widget = StatefulImage::new(None);
            f.render_stateful_widget(image_widget, inner, protocol);
        } else {
            let paragraph = Paragraph::new("Image data unavailable.").style(theme.base_style());
            f.render_widget(paragraph, inner);
        }
    }

    pub fn render_bookmarks_modal(&mut self, f: &mut Frame, area: Rect, theme: &Theme) {
        let modal_area = centered_rect(70, 60, area);
        f.render_widget(Clear, modal_area);

        let items: Vec<ListItem> = self
            .bookmark_items
            .iter()
            .map(|b| {
                let text = format!(" 📍 {}", b.label);
                ListItem::new(text).style(theme.base_style())
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Bookmarks (d: delete) "),
            )
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
        let modal_area = centered_rect(65, 70, area);
        f.render_widget(Clear, modal_area);

        let help_text = r#" Keybindings:
  j / Down         Scroll down 1 line
  k / Up           Scroll up 1 line
  Ctrl+F / Ctrl+B  Scroll 1 page down / up
  Ctrl+D / Ctrl+U  Scroll 1/2 page down / up
  gg / G           Go to top / bottom
  /                Search query (Up/Down for search history)
  n / N            Next / Previous match
  :                Command mode (Up/Down for command history)
  t                Table of Contents
  b / B            Add / List Bookmarks (d to delete a bookmark)
  v                View image at cursor line
  W                Toggle Widescreen / Centered Column
  J                Toggle Text Justification
  S                Toggle Simplified Mode
  C                Toggle CSS Styling
  q / Esc          Back / Quit

 Library view:
  d                Delete selected book
  r                Cycle sort order (Recent/Title/Author)
  /                Filter library by title/author
  :scan <dir>      Recursively import books from a directory
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
