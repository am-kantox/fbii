# Layout Engine & Search Architecture

`tabook` separates document parsing from terminal line wrapping and search indexing.

## Document Layout Engine (`src/renderer/layout.rs`)

1. **`BookLayout::build(book, config, simplified_mode)`**:
   - Converts format-neutral `Block` tree into formatted, line-wrapped `WrappedLine` instances.
   - Calculates line width using `unicode-width` (correctly handling multi-byte Unicode and East Asian wide characters).
   - Applies paragraph indent (`paragraph_indent`), spacing (`paragraph_spacing`), and measure limit (`measure`).
   - Tracks precise character start/end offsets (`char_start`, `char_end`) for viewport scroll tracking.

2. **Simplified Mode (`src/renderer/simplify.rs`)**:
   - Strips nested quotes, table boxes, and complex block elements down to simple text paragraphs while preserving inline emphasis (bold, italic).

## Search Indexing (`src/search/index.rs`)

1. **NFKD Unicode Folding**:
   - Decomposes combined Unicode characters (`nfkd()`) and strips diacritics.
   - Converts strings to lower case (e.g. `İstanbul` $\rightarrow$ `istanbul`, `CAFÉ` $\rightarrow$ `cafe`).

2. **Matching & Highlights**:
   - Returns matching block indices and character range offsets.
   - Generates contextual snippets surrounding matches.
