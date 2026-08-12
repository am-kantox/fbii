use fbii::formats::epub::parse_epub;
use fbii::formats::fb2::parse_fb2_bytes;
use fbii::formats::model::Block;
use fbii::formats::parse_book_file;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use zip::write::SimpleFileOptions;

#[test]
fn test_fb2_parser_xml_decoding_and_structure() {
    let fb2_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <book-title>Test FB2 Book</book-title>
      <author>
        <first-name>Arthur</first-name>
        <last-name>Conan Doyle</last-name>
      </author>
      <sequence name="Sherlock Holmes" number="1"/>
    </title-info>
  </description>
  <body>
    <title><p>Title Chapter</p></title>
    <section>
      <title><p>Section 1 Title</p></title>
      <p>Sooner or later it was <strong>bound</strong> to happen.</p>
      <cite><p>A famous quote</p></cite>
      <poem>
        <stanza>
          <v>Line 1 of poem</v>
          <v>Line 2 of poem</v>
        </stanza>
      </poem>
    </section>
  </body>
</FictionBook>"#;

    let book = parse_fb2_bytes(fb2_xml.as_bytes(), "/fake/path.fb2").unwrap();
    assert_eq!(book.metadata.title, "Test FB2 Book");
    assert_eq!(book.metadata.authors, vec!["Arthur Conan Doyle"]);
    assert_eq!(book.metadata.series_name.unwrap(), "Sherlock Holmes");
    assert_eq!(book.metadata.series_index.unwrap(), 1);

    assert!(book.content.len() >= 3);
    match &book.content[2] {
        Block::Paragraph(inlines) => {
            assert_eq!(inlines[0].plain_text(), "Sooner or later it was ");
            assert_eq!(
                inlines[1],
                fbii::formats::model::Inline::Bold(vec![fbii::formats::model::Inline::Text(
                    "bound".to_string()
                )])
            );
        }
        _ => panic!("Expected paragraph at block 2"),
    }
}

#[test]
fn test_fb2_windows_1251_decoding() {
    let fb2_xml = r#"<?xml version="1.0" encoding="windows-1251"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <book-title>Cyrillic Test</book-title>
      <author><first-name>Тест</first-name><last-name>Автор</last-name></author>
    </title-info>
  </description>
  <body>
    <section><p>Проверка кодировки</p></section>
  </body>
</FictionBook>"#;

    let (win1251_bytes, _, _) = encoding_rs::WINDOWS_1251.encode(fb2_xml);
    let book = parse_fb2_bytes(&win1251_bytes, "/fake/win1251.fb2").unwrap();
    assert_eq!(book.metadata.title, "Cyrillic Test");
    assert_eq!(book.metadata.authors, vec!["Тест Автор"]);
}

#[test]
fn test_fb2_zip_parser() {
    let temp_file = NamedTempFile::new().unwrap();
    let zip_path = temp_file.path().with_extension("fb2.zip");

    let fb2_content = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info>
      <book-title>Zipped FB2</book-title>
      <author><first-name>John</first-name><last-name>Doe</last-name></author>
    </title-info>
  </description>
  <body><section><p>Zipped content paragraph.</p></section></body>
</FictionBook>"#;

    {
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("book.fb2", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(fb2_content.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let book = parse_book_file(&zip_path).unwrap();
    assert_eq!(book.metadata.title, "Zipped FB2");
    assert_eq!(book.content.len(), 1);

    let _ = std::fs::remove_file(zip_path);
}

#[test]
fn test_epub_parser() {
    let temp_file = NamedTempFile::new().unwrap();
    let epub_path = temp_file.path().with_extension("epub");

    {
        let file = std::fs::File::create(&epub_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        zip.start_file("META-INF/container.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
   <rootfiles>
      <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
   </rootfiles>
</container>"#
                .as_bytes(),
        )
        .unwrap();

        zip.start_file("OEBPS/content.opf", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Sample EPUB</dc:title>
    <dc:creator>Jane Austen</dc:creator>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#
                .as_bytes(),
        )
        .unwrap();

        zip.start_file("OEBPS/chapter1.xhtml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Chapter 1</title></head>
<body>
  <h1>Chapter One</h1>
  <p>It is a truth universally acknowledged...</p>
</body>
</html>"#.as_bytes(),
        ).unwrap();

        zip.finish().unwrap();
    }

    let book = parse_epub(&epub_path).unwrap();
    assert_eq!(book.metadata.title, "Sample EPUB");
    assert_eq!(book.metadata.authors, vec!["Jane Austen"]);
    assert!(book.content.len() >= 2);

    let _ = std::fs::remove_file(epub_path);
}

#[test]
fn test_unsupported_file_extension_error() {
    let res = parse_book_file(Path::new("/tmp/nonexistent_file.pdf"));
    assert!(res.is_err());
}

#[test]
fn test_parse_book_uri_file_and_percent_decoding() {
    let temp_file = NamedTempFile::new().unwrap();
    let fb2_path = temp_file.path().with_extension("fb2");
    let fb2_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info>
      <book-title>URI Book</book-title>
      <author><first-name>Test</first-name><last-name>Author</last-name></author>
    </title-info>
  </description>
  <body><section><p>URI content.</p></section></body>
</FictionBook>"#;

    std::fs::write(&fb2_path, fb2_xml).unwrap();

    let uri_string = format!("file://{}", fb2_path.display());
    let book = fbii::formats::parse_book_uri(&uri_string).unwrap();
    assert_eq!(book.metadata.title, "URI Book");

    let _ = std::fs::remove_file(fb2_path);
}

#[test]
fn test_fixtures_epub_embedded_image() {
    let epub_path = Path::new("fixtures/test.epub");
    let book = parse_book_file(epub_path).expect("fixtures/test.epub should parse successfully");

    assert_eq!(book.metadata.title, "Test EPUB");
    
    // Check content has image block
    let has_image_block = book.content.iter().any(|b| matches!(b, Block::Image { .. }));
    assert!(has_image_block, "fixtures/test.epub should contain a Block::Image");

    // Check resources has decodable image
    assert!(!book.resources.is_empty(), "fixtures/test.epub should load image resources");
    let img_bytes = book.resources.values().next().expect("Should have at least 1 resource");
    let decoded = image::load_from_memory(img_bytes);
    assert!(decoded.is_ok(), "Image resource in test.epub should be valid decodable image data");

    // Check layout includes image key
    let config = fbii::config::TypographyConfig::default();
    let layout = fbii::renderer::BookLayout::build(&book, &config, false);
    let has_layout_image = layout.lines.iter().any(|l| l.image_key.is_some());
    assert!(has_layout_image, "BookLayout should include lines with image_key for test.epub");
}

#[test]
fn test_fixtures_fb2_embedded_image() {
    let fb2_path = Path::new("fixtures/test.fb2");
    let book = parse_book_file(fb2_path).expect("fixtures/test.fb2 should parse successfully");

    assert_eq!(book.metadata.title, "Test FB2");

    // Check content has image block
    let has_image_block = book.content.iter().any(|b| matches!(b, Block::Image { .. }));
    assert!(has_image_block, "fixtures/test.fb2 should contain a Block::Image");

    // Check resources has decodable binary object
    assert!(book.resources.contains_key("binary001"), "fixtures/test.fb2 resources should contain 'binary001'");
    let img_bytes = &book.resources["binary001"];
    let decoded = image::load_from_memory(img_bytes);
    assert!(decoded.is_ok(), "Binary object in test.fb2 should be valid decodable image data");

    // Check layout includes image key
    let config = fbii::config::TypographyConfig::default();
    let layout = fbii::renderer::BookLayout::build(&book, &config, false);
    let has_layout_image = layout.lines.iter().any(|l| l.image_key.as_deref() == Some("binary001"));
    assert!(has_layout_image, "BookLayout should include lines with image_key='binary001' for test.fb2");
}
