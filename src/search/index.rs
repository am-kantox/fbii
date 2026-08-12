use crate::formats::model::Book;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub block_index: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct BookSearchIndex {
    pub blocks_text: Vec<String>,
    pub normalized_blocks: Vec<String>,
    pub block_char_offsets: Vec<usize>,
}

impl BookSearchIndex {
    pub fn build(book: &Book) -> Self {
        let mut blocks_text = Vec::new();
        let mut normalized_blocks = Vec::new();
        let mut block_char_offsets = Vec::new();

        let mut cumulative_char_offset = 0;

        for block in &book.content {
            let plain = block.plain_text();
            let norm = fold_str(&plain);

            block_char_offsets.push(cumulative_char_offset);
            cumulative_char_offset += plain.chars().count();

            blocks_text.push(plain);
            normalized_blocks.push(norm);
        }

        Self {
            blocks_text,
            normalized_blocks,
            block_char_offsets,
        }
    }

    pub fn search(&self, query: &str) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        let folded_query = fold_str(query);
        if folded_query.is_empty() {
            return matches;
        }

        let query_char_count = query.chars().count();

        for (block_idx, norm_text) in self.normalized_blocks.iter().enumerate() {
            let block_base = self.block_char_offsets[block_idx];
            let mut start_byte = 0;

            while let Some(idx) = norm_text[start_byte..].find(&folded_query) {
                let match_byte_start = start_byte + idx;
                let match_byte_end = match_byte_start + folded_query.len();

                let block_char_start = norm_text[..match_byte_start].chars().count();
                let char_start = block_base + block_char_start;
                let char_end = char_start + query_char_count;

                let original = &self.blocks_text[block_idx];
                let snippet: String = original
                    .chars()
                    .skip(block_char_start.saturating_sub(10))
                    .take(query_char_count + 20)
                    .collect();

                matches.push(SearchMatch {
                    block_index: block_idx,
                    char_start,
                    char_end,
                    snippet,
                });

                start_byte = match_byte_end;
            }
        }

        matches
    }
}

pub fn fold_str(s: &str) -> String {
    s.nfkd()
        .filter(|c| !c_is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

fn c_is_combining_mark(c: char) -> bool {
    // Unicode combining character range heuristics
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1DC0}'..='\u{1DFF}' | '\u{20D0}'..='\u{20FF}' | '\u{FE20}'..='\u{FE2F}')
}
