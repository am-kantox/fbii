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

        for (block_idx, block) in book.content.iter().enumerate() {
            let block_lines = layout_block(
                block,
                block_idx,
                measure,
                indent,
                simplified_mode,
                config.hyphenation,
                &mut current_char_offset,
            );

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
                    });
                }
            }
        }

        Self { lines }
    }

    pub fn line_at_char_offset(&self, char_offset: usize) -> usize {
        for (i, line) in self.lines.iter().enumerate() {
            if char_offset >= line.char_start && char_offset <= line.char_end {
                return i;
            }
        }
        self.lines.len().saturating_sub(1)
    }
}

fn layout_block(
    block: &Block,
    block_idx: usize,
    measure: usize,
    indent: usize,
    simplified_mode: bool,
    _hyphenation: bool,
    char_offset: &mut usize,
) -> Vec<WrappedLine> {
    let mut result = Vec::new();

    match block {
        Block::Paragraph(inlines) => {
            let spans = flatten_inlines(inlines, false, 0, false);
            let block_lines = wrap_spans_into_lines(spans, measure, indent, block_idx, char_offset);
            result.extend(block_lines);
        }
        Block::Heading { level, inlines } => {
            let spans = flatten_inlines(inlines, true, *level, false);
            let block_lines = wrap_spans_into_lines(spans, measure, 0, block_idx, char_offset);
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
                    _hyphenation,
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
                let item_lines =
                    wrap_spans_into_lines(item_spans, measure, 2, block_idx, char_offset);
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
                });
            }
        }
        Block::Poem { stanzas } => {
            for stanza in stanzas {
                for line_inlines in &stanza.lines {
                    let spans = flatten_inlines(line_inlines, false, 0, false);
                    let line_wrapped =
                        wrap_spans_into_lines(spans, measure, 4, block_idx, char_offset);
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
            });
        }
        Block::Empty => {
            result.push(WrappedLine {
                spans: Vec::new(),
                block_index: block_idx,
                char_start: *char_offset,
                char_end: *char_offset,
                is_empty_line: true,
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
                lines.push(WrappedLine {
                    spans: current_line_spans,
                    block_index: block_idx,
                    char_start: line_start_offset,
                    char_end: *char_offset,
                    is_empty_line: false,
                });
                current_line_spans = Vec::new();
                current_width = 0;
                line_start_offset = *char_offset;
            }

            *char_offset += word.chars().count();
            current_width += word_width;

            let mut word_span = span.clone();
            word_span.text = word.to_string();
            current_line_spans.push(word_span);
        }
    }

    if !current_line_spans.is_empty() {
        lines.push(WrappedLine {
            spans: current_line_spans,
            block_index: block_idx,
            char_start: line_start_offset,
            char_end: *char_offset,
            is_empty_line: false,
        });
    }

    lines
}
