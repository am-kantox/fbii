# Format Parsers & Normalization Rules

`tabook` parses e-book files into a format-neutral document representation defined in `src/formats/model.rs`.

## Supported Formats

| Format | File Extensions | Parser Module | Parsing Strategy |
| :--- | :--- | :--- | :--- |
| **FB2** | `.fb2`, `.xml` | `src/formats/fb2.rs` | XML DOM parsing using `roxmltree` + `encoding_rs` character decoding. Extracts `<description>` metadata, base64 `<binary>` image resources, `<section>` titles/body blocks. |
| **FB2.ZIP** | `.fb2.zip` | `src/formats/fb2.rs` | ZIP archive extraction via `zip` crate. Locates embedded `.fb2` / `.xml` file and parses as FB2. |
| **EPUB** | `.epub` | `src/formats/epub.rs` | Standard EPUB 2.x/3.x extraction: parses `META-INF/container.xml` to locate `.opf`, reads manifest & spine, parses NCX/NAV Table of Contents, extracts chapter XHTML blocks into `Block` tree. |

## Normalization Pipeline

1. **Encoding Auto-detection (`src/formats/encoding.rs`)**:
   - Detects UTF-8 BOM, UTF-16 BOM, and `<?xml ... encoding="..." ?>` header declarations.
   - Decodes bytes using `encoding_rs` (supports Windows-1251, ISO-8859-1, UTF-8, etc.).

2. **Block Types**:
   - `Paragraph(Vec<Inline>)`: Paragraphs of inline elements.
   - `Heading { level: u8, inlines: Vec<Inline> }`: H1-H6 headers.
   - `Quote(Vec<Block>)`: Block quotes and cites.
   - `Epigraph(Vec<Block>)`: Chapter epigraphs.
   - `List { ordered: bool, items: Vec<ListItem> }`: Ordered and unordered lists.
   - `Table { rows: Vec<TableRow> }`: Structured tables.
   - `Poem { stanzas: Vec<PoemStanza> }`: Verse stanzas and poem lines.
   - `Image { key: String, alt: Option<String> }`: Inline or block image references matching `resources` HashMap.
   - `Empty`: Paragraph spacing / empty line breaks.
