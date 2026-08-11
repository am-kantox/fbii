use std::io::Write;
use tabook::formats::{parse_book_file, parse_fb2_bytes, Block, BookFormat, Inline};
use tempfile::NamedTempFile;
use zip::write::SimpleFileOptions;

#[test]
fn test_fb2_parser_xml_decoding_and_structure() {
    let fb2_xml = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
  <description>
    <title-info>
      <genre>sci-fi</genre>
      <author><first-name>Arthur</first-name><last-name>Clarke</last-name></author>
      <book-title>Rendezvous with Rama</book-title>
      <sequence name="Rama" number="1"/>
      <annotation><p>A space classic.</p></annotation>
      <coverpage><image l:href="#cover.jpg"/></coverpage>
    </title-info>
  </description>
  <body>
    <title><p>Rendezvous with Rama</p></title>
    <section>
      <title><p>Chapter 1: Spaceguard</p></title>
      <p>Sooner or later it was <strong>bound</strong> to happen.</p>
      <cite><p>Wisdom quote</p></cite>
      <poem>
        <stanza>
          <v>Line 1 of poem</v>
          <v>Line 2 of poem</v>
        </stanza>
      </poem>
      <table>
        <tr><th>Header</th></tr>
        <tr><td>Data cell</td></tr>
      </table>
    </section>
  </body>
  <binary id="cover.jpg" content-type="image/jpeg">aGVsbG8gd29ybGQ=</binary>
</FictionBook>
"##;

    let book = parse_fb2_bytes(fb2_xml.as_bytes(), "test.fb2").unwrap();
    assert_eq!(book.metadata.title, "Rendezvous with Rama");
    assert_eq!(book.metadata.authors, vec!["Arthur Clarke"]);
    assert_eq!(book.metadata.series_name, Some("Rama".to_string()));
    assert_eq!(book.metadata.series_index, Some(1));
    assert_eq!(book.metadata.genres, vec!["sci-fi"]);
    assert_eq!(book.metadata.cover_image_key, Some("cover.jpg".to_string()));
    assert_eq!(book.resources.get("cover.jpg").unwrap(), b"hello world");

    // TOC check
    assert_eq!(book.toc.len(), 2);
    assert_eq!(book.toc[0].title, "Rendezvous with Rama");
    assert_eq!(book.toc[1].title, "Chapter 1: Spaceguard");

    // Content check
    assert!(book.content.len() >= 5);
    match &book.content[2] {
        Block::Paragraph(inlines) => {
            assert_eq!(inlines[0].plain_text(), "Sooner or later it was ");
            assert_eq!(inlines[1], Inline::Bold(vec![Inline::Text("bound".to_string())]));
        }
        _ => panic!("Expected paragraph at block 2"),
    }
}

#[test]
fn test_fb2_zip_parser() {
    let temp_file = NamedTempFile::new().unwrap();
    let zip_path = temp_file.path().with_extension("fb2.zip");

    {
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file("book.fb2", options).unwrap();

        let fb2_xml = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info><book-title>Zipped Book</book-title></title-info>
  </description>
  <body><section><p>Test zipped content</p></section></body>
</FictionBook>"##;

        zip.write_all(fb2_xml.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let book = parse_book_file(&zip_path).unwrap();
    assert_eq!(book.metadata.title, "Zipped Book");
    assert_eq!(book.metadata.format, BookFormat::Fb2Zip);
    assert_eq!(book.content[0].plain_text(), "Test zipped content");

    let _ = std::fs::remove_file(zip_path);
}

#[test]
fn test_epub_parser() {
    let temp_file = NamedTempFile::new().unwrap();
    let epub_path = temp_file.path().with_extension("epub");

    {
        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        // 1. mimetype
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        // 2. META-INF/container.xml
        zip.start_file("META-INF/container.xml", options).unwrap();
        let container = r##"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"##;
        zip.write_all(container.as_bytes()).unwrap();

        // 3. OEBPS/content.opf
        zip.start_file("OEBPS/content.opf", options).unwrap();
        let opf = r##"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>EPUB Test Book</dc:title>
    <dc:creator>Jane Doe</dc:creator>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
  </spine>
</package>"##;
        zip.write_all(opf.as_bytes()).unwrap();

        // 4. OEBPS/toc.ncx
        zip.start_file("OEBPS/toc.ncx", options).unwrap();
        let ncx = r##"<?xml version="1.0"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <navMap>
    <navPoint id="np1">
      <navLabel><text>Chapter 1</text></navLabel>
      <content src="ch1.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"##;
        zip.write_all(ncx.as_bytes()).unwrap();

        // 5. OEBPS/ch1.xhtml
        zip.start_file("OEBPS/ch1.xhtml", options).unwrap();
        let ch1 = r##"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter 1</title></head>
  <body>
    <h1>Chapter 1 Header</h1>
    <p>Hello EPUB <em>world</em>!</p>
  </body>
</html>"##;
        zip.write_all(ch1.as_bytes()).unwrap();

        zip.finish().unwrap();
    }

    let book = parse_book_file(&epub_path).unwrap();
    assert_eq!(book.metadata.title, "EPUB Test Book");
    assert_eq!(book.metadata.authors, vec!["Jane Doe"]);
    assert_eq!(book.metadata.format, BookFormat::Epub);
    assert_eq!(book.toc[0].title, "Chapter 1");
    assert_eq!(book.content[0].plain_text(), "Chapter 1 Header");
    assert_eq!(book.content[1].plain_text(), "Hello EPUB world!");

    let _ = std::fs::remove_file(epub_path);
}
