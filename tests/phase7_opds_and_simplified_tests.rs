use fbii::config::Config;
use fbii::db::LibraryDb;
use fbii::opds::{parse_opds_feed, resolve_url, OpdsLinkType};
use fbii::tui::App;

#[test]
fn test_opds_feed_parsing_and_resolution() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Project Gutenberg Test Catalog</title>
  <link rel="next" href="/ebooks/search.opds/?start_index=26"/>
  <entry>
    <title>Pride and Prejudice</title>
    <author><name>Jane Austen</name></author>
    <content type="text">Jane Austen</content>
    <link rel="http://opds-spec.org/acquisition" type="application/epub+zip" href="/ebooks/1342.epub.images"/>
  </entry>
  <entry>
    <title>Classics Sub-Catalog</title>
    <author><name>Various</name></author>
    <link rel="subsection" type="application/atom+xml;profile=opds-catalog" href="/ebooks/subject/classics.opds"/>
  </entry>
</feed>"#;

    let base_url = "https://www.gutenberg.org/ebooks/search.opds/";
    let feed = parse_opds_feed(xml, base_url).unwrap();

    assert_eq!(feed.title, "Project Gutenberg Test Catalog");
    assert_eq!(
        feed.next_url.unwrap(),
        "https://www.gutenberg.org/ebooks/search.opds/?start_index=26"
    );
    assert_eq!(feed.entries.len(), 2);

    assert_eq!(feed.entries[0].title, "Pride and Prejudice");
    assert_eq!(feed.entries[0].author, "Jane Austen");
    assert_eq!(
        feed.entries[0].link,
        OpdsLinkType::Acquisition("https://www.gutenberg.org/ebooks/1342.epub.images".to_string())
    );

    assert_eq!(feed.entries[1].title, "Classics Sub-Catalog");
    assert_eq!(
        feed.entries[1].link,
        OpdsLinkType::Catalog("https://www.gutenberg.org/ebooks/subject/classics.opds".to_string())
    );
}

#[test]
fn test_url_resolution_helper() {
    assert_eq!(
        resolve_url(
            "https://www.gutenberg.org/ebooks/opds/",
            "/ebooks/1342.epub"
        ),
        "https://www.gutenberg.org/ebooks/1342.epub"
    );
    assert_eq!(
        resolve_url("https://example.com/catalog/", "sub.opds"),
        "https://example.com/catalog/sub.opds"
    );
    assert_eq!(
        resolve_url(
            "https://example.com/catalog/",
            "https://other.com/book.epub"
        ),
        "https://other.com/book.epub"
    );
}

#[tokio::test]
async fn test_opds_app_catalogs_and_simplified_toggle() {
    let config = Config::default();
    let db = LibraryDb::new_in_memory().await.unwrap();
    let mut app = App::new(
        config,
        db,
        std::path::PathBuf::from("/tmp/fbii_test_config.toml"),
    );

    // Verify default OPDS catalog for Gutenberg exists
    assert!(app.opds_catalogs.contains_key("gutenberg"));
    assert_eq!(
        app.opds_catalogs.get("gutenberg").unwrap(),
        "https://www.gutenberg.org/ebooks/search.opds/"
    );

    // Add custom catalog
    app.opds_catalogs.insert(
        "mycatalog".to_string(),
        "https://mycatalog.org/feed.opds".to_string(),
    );
    assert_eq!(
        app.opds_catalogs.get("mycatalog").unwrap(),
        "https://mycatalog.org/feed.opds"
    );

    // Verify simplified mode toggle
    assert!(!app.config.display.simplified_mode);
    app.config.display.simplified_mode = true;
    assert!(app.config.display.simplified_mode);
}
