use fbii::db::LibraryDb;
use fbii::formats::model::{Book, BookFormat, Metadata};

#[tokio::test]
async fn test_database_book_upsert_and_queries() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let metadata = Metadata {
        title: "Dune".to_string(),
        authors: vec!["Frank Herbert".to_string()],
        series_name: Some("Dune Chronicles".to_string()),
        series_index: Some(1),
        format: BookFormat::Epub,
        ..Default::default()
    };

    let book = Book::new("dune-123", "/books/dune.epub", metadata);

    // Upsert book
    db.upsert_book(&book, 1500, 25.5).await.unwrap();

    // Query book by ID
    let fetched = db.get_book_by_id("dune-123").await.unwrap().unwrap();
    assert_eq!(fetched.title, "Dune");
    assert_eq!(fetched.authors, "Frank Herbert");
    assert_eq!(fetched.progress_offset, 1500);
    assert_eq!(fetched.progress_percent, 25.5);

    // Query book by path
    let by_path = db
        .get_book_by_path("/books/dune.epub")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_path.id, "dune-123");

    // List books
    let books = db.list_books().await.unwrap();
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].id, "dune-123");
}

#[tokio::test]
async fn test_database_get_nonexistent_book() {
    let db = LibraryDb::new_in_memory().await.unwrap();
    let res = db.get_book_by_id("non-existent-id").await.unwrap();
    assert!(res.is_none());

    let res_path = db.get_book_by_path("/non/existent/path.fb2").await.unwrap();
    assert!(res_path.is_none());
}

#[tokio::test]
async fn test_database_delete_book_cascade() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let metadata = Metadata {
        title: "Delete Target".to_string(),
        ..Default::default()
    };

    let book = Book::new("del-1", "/books/del.fb2", metadata);
    db.upsert_book(&book, 0, 0.0).await.unwrap();

    // Add bookmark
    db.add_bookmark("del-1", 42, "Note").await.unwrap();
    assert_eq!(db.list_bookmarks("del-1").await.unwrap().len(), 1);

    // Delete book
    db.delete_book("del-1").await.unwrap();

    assert!(db.get_book_by_id("del-1").await.unwrap().is_none());
    assert_eq!(db.list_bookmarks("del-1").await.unwrap().len(), 0);
}

#[tokio::test]
async fn test_database_bookmarks_and_sessions() {
    let db = LibraryDb::new_in_memory().await.unwrap();

    let book = Book::new("b1", "/path/book.fb2", Metadata::default());
    db.upsert_book(&book, 0, 0.0).await.unwrap();

    // Add bookmarks
    let bm1 = db.add_bookmark("b1", 100, "Chapter 1 start").await.unwrap();
    let bm2 = db
        .add_bookmark("b1", 500, "Interesting quote")
        .await
        .unwrap();

    let bookmarks = db.list_bookmarks("b1").await.unwrap();
    assert_eq!(bookmarks.len(), 2);
    assert_eq!(bookmarks[0].id, bm1.id);

    db.delete_bookmark(&bm1.id.to_string()).await.unwrap();
    let updated = db.list_bookmarks("b1").await.unwrap();
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].id, bm2.id);

    // Reading sessions
    let session_id = db.start_reading_session("b1").await.unwrap();
    db.end_reading_session(&session_id, 25).await.unwrap();

    db.record_history("b1").await.unwrap();
}
