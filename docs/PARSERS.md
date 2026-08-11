# Format Parsers & Encoding Normalization

`fbii` parses e-book files into a format-neutral document representation defined in `src/formats/model.rs`.

## Supported Formats

1. **FictionBook 2.0 / 2.1 (`.fb2`)**:
   - Parsed with `roxmltree`.
   - Extracts metadata (title, author, series, annotation, genres, coverpage).
   - Extracts body sections, block quotes, epigraphs, poems, and base64 encoded binary images.

2. **Zipped FictionBook (`.fb2.zip`)**:
   - Reads ZIP container via `zip` crate.
   - Parses contained XML file using `parse_fb2_bytes`.

3. **EPUB 2.x / 3.x (`.epub`)**:
   - Parses `META-INF/container.xml` to find `.opf` package root.
   - Extracts OPF metadata, manifest items, and spine reading order.
   - Extracts Table of Contents from NCX (`toc.ncx`) or EPUB 3 Navigation Document (`nav.xhtml`).
   - Strips DOCTYPE declarations to prevent XML parser DTD resolution errors.

## Encoding & Character Sets (`src/formats/encoding.rs`)

- Checks BOM bytes (`UTF-8`, `UTF-16LE`, `UTF-16BE`).
- Parses XML header declarations (e.g. `<?xml version="1.0" encoding="windows-1251"?>`).
- Decodes non-UTF8 text into native Rust `String` using `encoding_rs`.
