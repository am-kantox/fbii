use crate::formats::model::{Block, Inline};

pub fn simplify_blocks(blocks: &[Block]) -> Vec<Block> {
    let mut result = Vec::new();

    for block in blocks {
        match block {
            Block::Paragraph(inlines) => {
                result.push(Block::Paragraph(simplify_inlines(inlines)));
            }
            Block::Heading { level, inlines } => {
                result.push(Block::Heading {
                    level: *level,
                    inlines: simplify_inlines(inlines),
                });
            }
            Block::Quote(inner) | Block::Epigraph(inner) | Block::Annotation(inner) => {
                let simplified_inner = simplify_blocks(inner);
                result.extend(simplified_inner);
            }
            // CSS-hidden content is omitted by this context-free transform,
            // matching the typical default expectation for `display: none`.
            Block::Hidden(_) => {}
            Block::List { ordered, items } => {
                let mut simple_items = Vec::new();
                for item in items {
                    simple_items.push(crate::formats::model::ListItem {
                        inlines: simplify_inlines(&item.inlines),
                    });
                }
                result.push(Block::List {
                    ordered: *ordered,
                    items: simple_items,
                });
            }
            Block::Poem { stanzas } => {
                for stanza in stanzas {
                    for line in &stanza.lines {
                        result.push(Block::Paragraph(simplify_inlines(line)));
                    }
                }
            }
            Block::Table { rows } => {
                for row in rows {
                    let mut row_inlines = Vec::new();
                    for cell in &row.cells {
                        row_inlines.extend(simplify_inlines(&cell.inlines));
                        row_inlines.push(Inline::Text(" | ".to_string()));
                    }
                    if !row_inlines.is_empty() {
                        result.push(Block::Paragraph(row_inlines));
                    }
                }
            }
            Block::Image { alt, key } => {
                let label = alt.clone().unwrap_or_else(|| format!("[Image: {}]", key));
                result.push(Block::Paragraph(vec![Inline::Text(label)]));
            }
            Block::Empty => {
                result.push(Block::Empty);
            }
        }
    }

    result
}

fn simplify_inlines(inlines: &[Inline]) -> Vec<Inline> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => out.push(Inline::Text(t.clone())),
            Inline::Bold(inner) => out.push(Inline::Bold(simplify_inlines(inner))),
            Inline::Italic(inner) => out.push(Inline::Italic(simplify_inlines(inner))),
            Inline::Underline(inner) => out.push(Inline::Underline(simplify_inlines(inner))),
            Inline::Strike(inner) => out.push(Inline::Strike(simplify_inlines(inner))),
            Inline::Code(t) => out.push(Inline::Code(t.clone())),
            Inline::Link { inlines: inner, .. } => out.extend(simplify_inlines(inner)),
            // CSS-hidden content is omitted by this context-free transform,
            // matching the typical default expectation for `display: none`.
            Inline::Hidden(_) => {}
            Inline::Image { alt, key } => {
                let label = alt.clone().unwrap_or_else(|| format!("[Image: {}]", key));
                out.push(Inline::Text(label));
            }
            Inline::LineBreak => out.push(Inline::LineBreak),
        }
    }
    out
}
