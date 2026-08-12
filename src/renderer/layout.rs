use crate::config::TypographyConfig;
use crate::formats::model::{Block, Book, Inline};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyledSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    pub is_heading: bool,
    pub heading_level: u8,
    pub is_quote: bool,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WrappedLine {
    pub spans: Vec<StyledSpan>,
    pub block_index: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub is_empty_line: bool,
    /// Set when this line renders a standalone `Block::Image`, holding the
    /// resource key into `Book::resources` so the reader can offer to
    /// display the actual image via the `ViewImage` action.
    pub image_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BookLayout {
    pub lines: Vec<WrappedLine>,
}

impl BookLayout {
    pub fn build(book: &Book, config: &TypographyConfig, simplified_mode: bool) -> Self {
        let mut lines = Vec::new();
        let measure = config.measure as usize;
        let indent = if simplified_mode {
            0
        } else {
            config.paragraph_indent as usize
        };
        let spacing = if simplified_mode {
            1
        } else {
            config.paragraph_spacing as usize
        };

        let mut current_char_offset = 0;
        let line_spacing = config.line_spacing as usize;

        for (block_idx, block) in book.content.iter().enumerate() {
            let block_lines = layout_block(
                block,
                block_idx,
                measure,
                indent,
                simplified_mode,
                config,
                &mut current_char_offset,
            );
            let block_lines = interleave_line_spacing(block_lines, line_spacing);

            lines.extend(block_lines);

            // Add paragraph spacing empty lines
            if spacing > 0 && block_idx < book.content.len() - 1 {
                for _ in 0..spacing {
                    lines.push(WrappedLine {
                        spans: Vec::new(),
                        block_index: block_idx,
                        char_start: current_char_offset,
                        char_end: current_char_offset,
                        is_empty_line: true,
                        image_key: None,
                    });
                }
            }
        }

        Self { lines }
    }

    /// Map a character offset to a line index. When `char_offset` falls
    /// exactly on a boundary shared by two adjacent lines (the common case
    /// when anchoring on a line's `char_start`, e.g. paragraph-spacing
    /// blank lines or the position-preserving layout rebuild), this
    /// consistently resolves to the *later* of the two lines — i.e. the
    /// last line whose `char_start` is at or before the offset — rather
    /// than whichever line happens to be encountered first.
    pub fn line_at_char_offset(&self, char_offset: usize) -> usize {
        if self.lines.is_empty() {
            return 0;
        }

        let mut best_idx = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if line.char_start <= char_offset {
                best_idx = i;
            } else {
                break;
            }
        }
        best_idx
    }

    /// Reading-progress percentage for a given scroll position, accounting
    /// for the visible viewport height. Shared by the reader status bar and
    /// by `App::save_progress` so both agree on the same value.
    pub fn progress_percent(&self, scroll_offset: usize, viewport_height: usize) -> f64 {
        let total_lines = self.lines.len();
        if total_lines == 0 {
            return 0.0;
        }
        ((scroll_offset + viewport_height).min(total_lines) as f64 / total_lines as f64) * 100.0
    }
}

fn layout_block(
    block: &Block,
    block_idx: usize,
    measure: usize,
    indent: usize,
    simplified_mode: bool,
    config: &TypographyConfig,
    char_offset: &mut usize,
) -> Vec<WrappedLine> {
    let mut result = Vec::new();

    match block {
        Block::Paragraph(inlines) => {
            let spans = flatten_inlines(inlines, false, 0, false);
            let block_lines = wrap_spans_into_lines(
                spans,
                measure,
                indent,
                block_idx,
                config.hyphenation,
                config.justified,
                char_offset,
            );
            result.extend(block_lines);
        }
        Block::Heading { level, inlines } => {
            let spans = flatten_inlines(inlines, true, *level, false);
            let block_lines =
                wrap_spans_into_lines(spans, measure, 0, block_idx, false, false, char_offset);
            result.extend(block_lines);
        }
        Block::Quote(blocks) | Block::Epigraph(blocks) | Block::Annotation(blocks) => {
            for b in blocks {
                let inner_lines = layout_block(
                    b,
                    block_idx,
                    measure.saturating_sub(4),
                    0,
                    simplified_mode,
                    config,
                    char_offset,
                );
                for mut line in inner_lines {
                    if !simplified_mode {
                        line.spans.insert(
                            0,
                            StyledSpan {
                                text: "│ ".to_string(),
                                is_quote: true,
                                ..Default::default()
                            },
                        );
                    }
                    result.push(line);
                }
            }
        }
        Block::List { ordered, items } => {
            for (i, item) in items.iter().enumerate() {
                let prefix = if *ordered {
                    format!("{}. ", i + 1)
                } else {
                    "• ".to_string()
                };
                let mut item_spans = vec![StyledSpan {
                    text: prefix,
                    bold: true,
                    ..Default::default()
                }];
                item_spans.extend(flatten_inlines(&item.inlines, false, 0, false));
                let item_lines = wrap_spans_into_lines(
                    item_spans,
                    measure,
                    2,
                    block_idx,
                    config.hyphenation,
                    config.justified,
                    char_offset,
                );
                result.extend(item_lines);
            }
        }
        Block::Table { rows } => {
            for row in rows {
                let row_str: String = row
                    .cells
                    .iter()
                    .map(|c| {
                        c.inlines
                            .iter()
                            .map(|i| i.plain_text())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                let start = *char_offset;
                *char_offset += row_str.chars().count();
                result.push(WrappedLine {
                    spans: vec![StyledSpan {
                        text: row_str,
                        code: true,
                        ..Default::default()
                    }],
                    block_index: block_idx,
                    char_start: start,
                    char_end: *char_offset,
                    is_empty_line: false,
                    image_key: None,
                });
            }
        }
        Block::Poem { stanzas } => {
            for stanza in stanzas {
                for line_inlines in &stanza.lines {
                    let spans = flatten_inlines(line_inlines, false, 0, false);
                    let line_wrapped = wrap_spans_into_lines(
                        spans,
                        measure,
                        4,
                        block_idx,
                        config.hyphenation,
                        false,
                        char_offset,
                    );
                    result.extend(line_wrapped);
                }
            }
        }
        Block::Image { key, alt } => {
            let label = alt.clone().unwrap_or_else(|| format!("[Image: {}]", key));
            let start = *char_offset;
            *char_offset += label.chars().count();
            result.push(WrappedLine {
                spans: vec![StyledSpan {
                    text: label,
                    italic: true,
                    ..Default::default()
                }],
                block_index: block_idx,
                char_start: start,
                char_end: *char_offset,
                is_empty_line: false,
                image_key: Some(key.clone()),
            });
        }
        Block::Empty => {
            result.push(WrappedLine {
                spans: Vec::new(),
                block_index: block_idx,
                char_start: *char_offset,
                char_end: *char_offset,
                is_empty_line: true,
                image_key: None,
            });
        }
    }

    result
}

fn flatten_inlines(
    inlines: &[Inline],
    is_heading: bool,
    heading_level: u8,
    is_quote: bool,
) -> Vec<StyledSpan> {
    let mut spans = Vec::new();

    for inline in inlines {
        match inline {
            Inline::Text(t) => {
                spans.push(StyledSpan {
                    text: t.clone(),
                    is_heading,
                    heading_level,
                    is_quote,
                    ..Default::default()
                });
            }
            Inline::Bold(inner) => {
                let inner_spans = flatten_inlines(inner, is_heading, heading_level, is_quote);
                for mut s in inner_spans {
                    s.bold = true;
                    spans.push(s);
                }
            }
            Inline::Italic(inner) => {
                let inner_spans = flatten_inlines(inner, is_heading, heading_level, is_quote);
                for mut s in inner_spans {
                    s.italic = true;
                    spans.push(s);
                }
            }
            Inline::Underline(inner) => {
                let inner_spans = flatten_inlines(inner, is_heading, heading_level, is_quote);
                for mut s in inner_spans {
                    s.underline = true;
                    spans.push(s);
                }
            }
            Inline::Strike(inner) => {
                let inner_spans = flatten_inlines(inner, is_heading, heading_level, is_quote);
                for mut s in inner_spans {
                    s.strike = true;
                    spans.push(s);
                }
            }
            Inline::Link {
                target,
                inlines: inner,
            } => {
                let inner_spans = flatten_inlines(inner, is_heading, heading_level, is_quote);
                for mut s in inner_spans {
                    s.link_target = Some(target.clone());
                    s.underline = true;
                    spans.push(s);
                }
            }
            Inline::Code(t) => {
                spans.push(StyledSpan {
                    text: t.clone(),
                    code: true,
                    is_heading,
                    heading_level,
                    is_quote,
                    ..Default::default()
                });
            }
            Inline::Image { key, alt } => {
                let label = alt.clone().unwrap_or_else(|| format!("[Img: {}]", key));
                spans.push(StyledSpan {
                    text: label,
                    italic: true,
                    ..Default::default()
                });
            }
            Inline::LineBreak => {
                spans.push(StyledSpan {
                    text: "\n".to_string(),
                    ..Default::default()
                });
            }
        }
    }

    spans
}

fn wrap_spans_into_lines(
    spans: Vec<StyledSpan>,
    max_width: usize,
    indent: usize,
    block_idx: usize,
    hyphenation: bool,
    justified: bool,
    char_offset: &mut usize,
) -> Vec<WrappedLine> {
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut current_width = 0;
    let mut line_start_offset = *char_offset;

    if indent > 0 {
        current_line_spans.push(StyledSpan {
            text: " ".repeat(indent),
            ..Default::default()
        });
        current_width += indent;
    }

    for span in spans {
        let words: Vec<&str> = span.text.split_inclusive(' ').collect();
        for word in words {
            let word_width = word.width();

            if current_width + word_width > max_width && !current_line_spans.is_empty() {
                // Apply justification if enabled and line has multiple spans
                if justified && current_line_spans.len() > 1 && current_width < max_width {
                    let gap = max_width.saturating_sub(current_width);
                    let num_gaps = current_line_spans.len() - 1;
                    let add_per_gap = gap / num_gaps;
                    let mut rem = gap % num_gaps;
                    for (i, line_span) in current_line_spans.iter_mut().enumerate() {
                        if i < num_gaps {
                            let extra = add_per_gap
                                + if rem > 0 {
                                    rem -= 1;
                                    1
                                } else {
                                    0
                                };
                            line_span.text.push_str(&" ".repeat(extra));
                        }
                    }
                }

                lines.push(WrappedLine {
                    spans: current_line_spans,
                    block_index: block_idx,
                    char_start: line_start_offset,
                    char_end: *char_offset,
                    is_empty_line: false,
                    image_key: None,
                });
                current_line_spans = Vec::new();
                current_width = 0;
                line_start_offset = *char_offset;
            }

            // Word-level hyphenation split for long words exceeding measure
            if hyphenation && word_width > max_width && word.chars().count() > 6 {
                let mid = word.chars().count() / 2;
                let part1: String = word.chars().take(mid).collect();
                let part2: String = word.chars().skip(mid).collect();

                *char_offset += part1.chars().count();
                let mut p1_span = span.clone();
                p1_span.text = format!("{}-", part1);
                current_line_spans.push(p1_span);

                lines.push(WrappedLine {
                    spans: current_line_spans,
                    block_index: block_idx,
                    char_start: line_start_offset,
                    char_end: *char_offset,
                    is_empty_line: false,
                    image_key: None,
                });

                current_line_spans = Vec::new();
                line_start_offset = *char_offset;
                *char_offset += part2.chars().count();
                current_width = part2.width();
                let mut p2_span = span.clone();
                p2_span.text = part2;
                current_line_spans.push(p2_span);
            } else {
                *char_offset += word.chars().count();
                current_width += word_width;

                let mut word_span = span.clone();
                word_span.text = word.to_string();
                current_line_spans.push(word_span);
            }
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(WrappedLine {
            spans: current_line_spans,
            block_index: block_idx,
            char_start: line_start_offset,
            char_end: *char_offset,
            is_empty_line: false,
            image_key: None,
        });
    }

    lines
}

/// Insert `spacing - 1` blank lines between each pair of consecutive wrapped
/// lines within a single block, implementing `TypographyConfig::line_spacing`.
/// A `spacing` of 0 or 1 is a no-op, matching the default (single-spaced)
/// behavior.
fn interleave_line_spacing(block_lines: Vec<WrappedLine>, spacing: usize) -> Vec<WrappedLine> {
    if spacing <= 1 || block_lines.len() <= 1 {
        return block_lines;
    }

    let last_idx = block_lines.len() - 1;
    let mut result = Vec::with_capacity(block_lines.len() * spacing);
    for (i, line) in block_lines.into_iter().enumerate() {
        let block_index = line.block_index;
        let char_pos = line.char_end;
        result.push(line);
        if i != last_idx {
            for _ in 0..spacing.saturating_sub(1) {
                result.push(WrappedLine {
                    spans: Vec::new(),
                    block_index,
                    char_start: char_pos,
                    char_end: char_pos,
                    is_empty_line: true,
                    image_key: None,
                });
            }
        }
    }
    result
}
