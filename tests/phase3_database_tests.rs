use tabook::db::LibraryDb;
use tabook::formats::model::{Book, BookFormat, Metadata};

#[tokio::test]
async fn test_database_book_upsert_and_queries() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let mut metadata = Metadata::default();
    metadata.title = "Dune".to_string();
    metadata.authors = vec!["Frank Herbert".to_string()];
    metadata.series_name = Some("Dune Chronicles".to_string());
    metadata.series_index = Some(1);
    metadata.format = BookFormat::Epub;

    let book = Book::new("dune-123", "/books/dune.epub", metadata);

    // Upsert book
    db.upsert_book(&book, 1500, 25.5).await.unwrap();

    // Query book by ID
    let fetched = db.get_book_by_id("dune-123").await.unwrap().unwrap();
    assert_eq!(fetched.title, "Dune");
    assert_eq!(fetched.authors, "Frank Herbert");
    assert_eq!(fetched.series_name, Some("Dune Chronicles".to_string()));
    assert_eq!(fetched.series_index, Some(1));
    assert_eq!(fetched.progress_offset, 1500);
    assert_eq!(fetched.progress_percent, 25.5);

    // Query book by path
    let by_path = db.get_book_by_path("/books/dune.epub").await.unwrap().unwrap();
    assert_eq!(by_path.id, "dune-123");

    // List books
    let list = db.list_books().await.unwrap();
    assert_eq!(list.len(), 1);

    // Update progress
    db.upsert_book(&book, 3000, 50.0).await.unwrap();
    let updated = db.get_book_by_id("dune-123").await.unwrap().unwrap();
    assert_eq!(updated.progress_offset, 3000);
    assert_eq!(updated.progress_percent, 50.0);

    // Delete book
    db.delete_book("dune-123").await.unwrap();
    assert!(db.get_book_by_id("dune-123").await.unwrap().is_none());
}

#[tokio::test]
async fn test_database_bookmarks_and_sessions() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let book = Book::new("b1", "/books/b1.fb2", Metadata::default());
    db.upsert_book(&book, 0, 0.0).await.unwrap();

    // Add bookmarks
    let bm1 = db.add_bookmark("b1", 100, "Chapter 1 start").await.unwrap();
    let bm2 = db.add_bookmark("b1", 500, "Interesting quote").await.unwrap();

    let bookmarks = db.list_bookmarks("b1").await.unwrap();
    assert_eq!(bookmarks.len(), 2);
    assert_eq!(bookmarks[0].label, "Chapter 1 start");
    assert_eq!(bookmarks[1].label, "Interesting quote");

    // Delete bookmark
    db.delete_bookmark(&bm1.id).await.unwrap();
    let remaining = db.list_bookmarks("b1").await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, bm2.id);

    // History
    db.record_history("b1").await.unwrap();

    // Reading session
    let session_id = db.start_reading_session("b1").await.unwrap();
    db.end_reading_session(&session_id, 15).await.unwrap();
}
