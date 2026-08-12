use crate::formats::model::Book;
use crate::renderer::{BookLayout, WrappedLine};
use crate::themes::Theme;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::collections::HashMap;

/// A contiguous run of visible lines (within the current viewport) that
/// belong to the same inline image, as detected by [`detect_image_runs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRun {
    /// Row index within the visible viewport (0-based) where this run
    /// starts.
    pub start_row: usize,
    pub image_key: String,
    /// Number of rows to actually render the image widget over. Excludes
    /// the trailing text caption row when that row is part of the visible
    /// run (so the caption stays readable below the image); when the run
    /// is truncated by the viewport edge (the caption isn't visible yet),
    /// every visible row of the run is used for the image instead.
    pub image_rows: usize,
}

/// Scan a slice of visible lines (already sliced to the current scroll
/// window) for runs of lines sharing the same `image_key`, and determine
/// how many rows of each run should be covered by the actual rendered
/// image widget vs. left for the text caption. Pure and independent of any
/// terminal/picker state, so it can be unit tested directly.
pub fn detect_image_runs(lines: &[&WrappedLine]) -> Vec<ImageRun> {
    let mut runs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let key = match &lines[i].image_key {
            Some(k) => k.clone(),
            None => {
                i += 1;
                continue;
            }
        };
        let start = i;
        let mut j = i;
        while j < lines.len() && lines[j].image_key.as_deref() == Some(key.as_str()) {
            j += 1;
        }
        let run_len = j - start;
        let last_is_caption = !lines[j - 1].is_empty_line;
        let image_rows = if last_is_caption {
            run_len.saturating_sub(1)
        } else {
            run_len
        };
        if image_rows > 0 {
            runs.push(ImageRun {
                start_row: start,
                image_key: key,
                image_rows,
            });
        }
        i = j;
    }
    runs
}

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
    /// Whether the full-size image "zoom" modal is currently shown.
    /// Images are normally rendered inline in the scrollable text flow
    /// (see `image_cache`/`detect_image_runs`); this modal is for viewing
    /// an image larger than the space reserved for it inline.
    pub show_image: bool,
    /// Resource key of the image currently displayed in the zoom modal.
    pub current_image_key: Option<String>,
    /// Decoded/resized image render state for the zoom modal. Not
    /// `Clone`/`Debug`, so it is kept out of any derived traits on
    /// `ReaderView`.
    pub image_state: Option<StatefulProtocol>,
    /// Decoded cover-art render state, populated when the Info modal opens
    /// (if the book has a `cover_image_key` resource and a picker is
    /// available), cleared when it closes.
    pub cover_image_state: Option<StatefulProtocol>,
    /// Reading-session stats for the active book, populated when the Info
    /// modal opens.
    pub reading_stats: Option<crate::db::ReadingStats>,
    /// Decoded/resized inline images, keyed by resource key, rendered
    /// directly into the scrollable text flow (see `detect_image_runs`).
    /// Cleared whenever a new book is loaded.
    pub image_cache: HashMap<String, StatefulProtocol>,
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
            cover_image_state: None,
            reading_stats: None,
            image_cache: HashMap::new(),
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
        mut image_picker: Option<&mut Picker>,
        search_matches: &[crate::search::SearchMatch],
        current_match_idx: usize,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(1)])
            .split(area);

        f.render_widget(ratatui::widgets::Clear, chunks[0]);
        f.render_widget(Block::default().style(theme.base_style()), chunks[0]);

        let viewport_height = chunks[0].height as usize;
        self.last_viewport_height = viewport_height;
        let visible_lines: Vec<&WrappedLine> = layout
            .lines
            .iter()
            .skip(self.scroll_offset)
            .take(viewport_height)
            .collect();

        let mut paragraph_lines = Vec::new();
        for line in &visible_lines {
            if line.is_empty_line {
                paragraph_lines.push(Line::from(""));
                continue;
            }

            let mut spans = Vec::new();
            let mut span_start_char = line.char_start;

            for styled in &line.spans {
                let span_len = styled.text.chars().count();
                let span_end_char = span_start_char + span_len;

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

                let has_matches = search_matches
                    .iter()
                    .any(|m| m.char_start < span_end_char && m.char_end > span_start_char);

                if !has_matches || span_len == 0 {
                    spans.push(Span::styled(styled.text.clone(), style));
                } else {
                    let chars: Vec<char> = styled.text.chars().collect();
                    let mut i = 0;
                    while i < chars.len() {
                        let char_pos = span_start_char + i;

                        let is_active_match = search_matches
                            .get(current_match_idx)
                            .is_some_and(|m| char_pos >= m.char_start && char_pos < m.char_end);

                        let is_any_match = search_matches
                            .iter()
                            .any(|m| char_pos >= m.char_start && char_pos < m.char_end);

                        let mut j = i + 1;
                        while j < chars.len() {
                            let next_pos = span_start_char + j;
                            let next_active = search_matches
                                .get(current_match_idx)
                                .is_some_and(|m| next_pos >= m.char_start && next_pos < m.char_end);
                            let next_any = search_matches
                                .iter()
                                .any(|m| next_pos >= m.char_start && next_pos < m.char_end);

                            if next_active != is_active_match || next_any != is_any_match {
                                break;
                            }
                            j += 1;
                        }

                        let chunk_text: String = chars[i..j].iter().collect();
                        let mut chunk_style = style;
                        if is_active_match {
                            chunk_style = chunk_style
                                .bg(theme.accent)
                                .fg(theme.background)
                                .add_modifier(Modifier::BOLD);
                        } else if is_any_match {
                            chunk_style = chunk_style
                                .bg(theme.highlight)
                                .fg(theme.background)
                                .add_modifier(Modifier::BOLD);
                        }

                        spans.push(Span::styled(chunk_text, chunk_style));
                        i = j;
                    }
                }

                span_start_char = span_end_char;
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

        // Overlay actual rendered images (when a graphics protocol is
        // available) on top of the blank rows reserved for them in the
        // layout, so images appear inline in the scrollable flow instead
        // of requiring the `v` "zoom" modal.
        for run in detect_image_runs(&visible_lines) {
            if run.start_row as u16 >= reader_area.height {
                continue;
            }
            let available_rows = reader_area.height.saturating_sub(run.start_row as u16);
            let widget_height = (run.image_rows as u16).min(available_rows);
            if widget_height == 0 {
                continue;
            }
            let widget_rect = Rect {
                x: reader_area.x,
                y: reader_area.y + run.start_row as u16,
                width: reader_area.width,
                height: widget_height,
            };

            if let Some(picker) = image_picker.as_deref_mut() {
                if !self.image_cache.contains_key(&run.image_key) {
                    if let Some(bytes) = book.resources.get(&run.image_key) {
                        if let Ok(dyn_img) = image::load_from_memory(bytes) {
                            let protocol = picker.new_resize_protocol(dyn_img);
                            self.image_cache.insert(run.image_key.clone(), protocol);
                        }
                    }
                }
                if let Some(protocol) = self.image_cache.get_mut(&run.image_key) {
                    let image_widget = StatefulImage::new(None);
                    f.render_stateful_widget(image_widget, widget_rect, protocol);
                }
            }
        }

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

    /// Render the currently-loaded image (if any) full-size in a centered
    /// modal, as a "zoom" action for images that don't fit well at the
    /// smaller size reserved for them inline. Falls back to a placeholder
    /// message when no image data is available.
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
        let modal_area = centered_rect(65, 75, area);
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
  i                Book info (metadata, cover art, reading stats)
  v                Zoom the image at cursor line to full size
  R                Toggle recent books / Cycle sort order
  J                Toggle text justification
  W                Toggle wide screen / centered column
  S                Toggle simplified mode
  C                Toggle CSS styling
  q / Esc          Back / Quit

 Library view:
  d                Delete selected book
  r / R            Toggle recent books / Cycle sort order
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

        let stats_line = match self.reading_stats {
            Some(stats) => format!(
                "\n Reading sessions: {} | Pages read: {}",
                stats.sessions, stats.total_pages
            ),
            None => String::new(),
        };

        let info_text = format!(
            " Title: {}\n Authors: {}\n Format: {}\n Series: {}\n Genres: {}\n Annotation: {}{}",
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
                .unwrap_or_else(|| "None".to_string()),
            stats_line
        );

        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(" Book Information ");
        let inner = outer_block.inner(modal_area);
        f.render_widget(outer_block, modal_area);

        if self.cover_image_state.is_some() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(20), Constraint::Min(1)])
                .split(inner);

            if let Some(protocol) = self.cover_image_state.as_mut() {
                let image_widget = StatefulImage::new(None);
                f.render_stateful_widget(image_widget, cols[0], protocol);
            }

            let paragraph = Paragraph::new(info_text).style(theme.base_style());
            f.render_widget(paragraph, cols[1]);
        } else {
            let paragraph = Paragraph::new(info_text).style(theme.base_style());
            f.render_widget(paragraph, inner);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::StyledSpan;

    fn blank_line(image_key: Option<&str>) -> WrappedLine {
        WrappedLine {
            spans: Vec::new(),
            block_index: 0,
            char_start: 0,
            char_end: 0,
            is_empty_line: true,
            image_key: image_key.map(|s| s.to_string()),
        }
    }

    fn caption_line(image_key: &str) -> WrappedLine {
        WrappedLine {
            spans: vec![StyledSpan {
                text: format!("[Image: {}]", image_key),
                ..Default::default()
            }],
            block_index: 0,
            char_start: 0,
            char_end: 10,
            is_empty_line: false,
            image_key: Some(image_key.to_string()),
        }
    }

    fn text_line() -> WrappedLine {
        WrappedLine {
            spans: vec![StyledSpan {
                text: "Some text".to_string(),
                ..Default::default()
            }],
            block_index: 0,
            char_start: 0,
            char_end: 9,
            is_empty_line: false,
            image_key: None,
        }
    }

    #[test]
    fn test_detect_image_runs_full_run_reserves_all_but_caption_row() {
        let text = text_line();
        let blanks = vec![blank_line(Some("cover.png")); 3];
        let caption = caption_line("cover.png");
        let lines: Vec<&WrappedLine> = std::iter::once(&text)
            .chain(blanks.iter())
            .chain(std::iter::once(&caption))
            .collect();

        let runs = detect_image_runs(&lines);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].start_row, 1);
        assert_eq!(runs[0].image_key, "cover.png");
        // 3 blank rows + 1 caption row = 4-row run; the caption row is
        // excluded from the image widget's height.
        assert_eq!(runs[0].image_rows, 3);
    }

    #[test]
    fn test_detect_image_runs_truncated_at_viewport_bottom_uses_all_visible_rows() {
        // The viewport ends before the caption row becomes visible; every
        // visible row of the run must be used for the image, since there
        // is no visible caption to preserve space for.
        let blanks = vec![blank_line(Some("cover.png")); 4];
        let lines: Vec<&WrappedLine> = blanks.iter().collect();

        let runs = detect_image_runs(&lines);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].image_rows, 4);
    }

    #[test]
    fn test_detect_image_runs_truncated_at_viewport_top_starts_at_row_zero() {
        // Scrolled to the middle of an image's reserved rows: the first
        // visible line is already mid-run.
        let blanks = vec![blank_line(Some("cover.png")); 2];
        let caption = caption_line("cover.png");
        let lines: Vec<&WrappedLine> = blanks.iter().chain(std::iter::once(&caption)).collect();

        let runs = detect_image_runs(&lines);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].image_rows, 2);
    }

    #[test]
    fn test_detect_image_runs_no_images_returns_empty() {
        let a = text_line();
        let b = blank_line(None);
        let lines: Vec<&WrappedLine> = vec![&a, &b];
        assert!(detect_image_runs(&lines).is_empty());
    }

    #[test]
    fn test_detect_image_runs_caption_only_yields_no_widget_rows() {
        // No blank rows visible (e.g. simplified mode's single-line
        // placeholder): nothing to render an image widget over.
        let caption = caption_line("cover.png");
        let lines: Vec<&WrappedLine> = vec![&caption];
        assert!(detect_image_runs(&lines).is_empty());
    }

    #[test]
    fn test_detect_image_runs_multiple_distinct_images() {
        let cap_a = caption_line("a.png");
        let blanks_b = vec![blank_line(Some("b.png")); 2];
        let cap_b = caption_line("b.png");
        let blank_a = blank_line(Some("a.png"));
        let lines: Vec<&WrappedLine> = vec![&blank_a, &cap_a]
            .into_iter()
            .chain(blanks_b.iter())
            .chain(std::iter::once(&cap_b))
            .collect();

        let runs = detect_image_runs(&lines);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].image_key, "a.png");
        assert_eq!(runs[0].start_row, 0);
        assert_eq!(runs[0].image_rows, 1);
        assert_eq!(runs[1].image_key, "b.png");
        assert_eq!(runs[1].start_row, 2);
        assert_eq!(runs[1].image_rows, 2);
    }
}
