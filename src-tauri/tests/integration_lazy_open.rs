//! Integration tests for the lazy-open fallback in `RenderingService`.
//!
//! The frontend mounts `PremiumSidebar` and fires `get_book_toc` /
//! `get_book_chapter` immediately after opening a book — racing
//! `open_book_renderer`, which runs a blocking adapter load. The query
//! commands now lazy-open the book from the `books` table when no renderer is
//! registered yet (`RenderingService::open_if_needed`), so a TOC/chapter
//! query no longer fails with `BookNotFound` just because the open finished
//! a moment later.
//!
//! Tauri `State` is not constructible in tests (see integration_native_open.rs),
//! so the service-level helper that the commands call is exercised directly.
//! `open_book` uses `tokio::task::block_in_place`, which panics on a
//! current_thread runtime, hence `flavor = "multi_thread"`.

use shiori::db::Database;
use shiori::services::rendering_service::RenderingService;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static BOOK_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn create_temp_db() -> (Database, PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!(
        "shiori_lazy_open_test_{}_{}",
        std::process::id(),
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db = Database::new(&temp_dir.join("test.db")).unwrap();
    (db, temp_dir)
}

fn insert_book(db: &Database, path: &std::path::Path, format: &str) -> i64 {
    let conn = db.get_connection().unwrap();
    let uuid = format!(
        "lazy-{}-{}",
        std::process::id(),
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    conn.execute(
        "INSERT INTO books (uuid, title, file_path, file_format, file_size) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            uuid,
            "Lazy Book",
            path.to_string_lossy().to_string(),
            format,
            1234
        ],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// A book that is NOT pre-opened must be lazy-opened from the DB before the
/// TOC query succeeds — this is the exact race the frontend hits when
/// `PremiumSidebar` queries before `open_book_renderer` finishes.
#[tokio::test(flavor = "multi_thread")]
async fn toc_lazily_opens_book_from_db() {
    let (db, temp_dir) = create_temp_db();
    let file = temp_dir.join("book.txt");
    fs::write(
        &file,
        "First Heading\n\nSome content for the first section.\n\nSecond Heading\n\nMore content.",
    )
    .unwrap();
    let book_id = insert_book(&db, &file, "txt");

    let service = RenderingService::new(64);
    assert!(!service.is_open(book_id), "book must not be open yet");

    // Simulates the query command's lazy-open step (get_book_toc without a
    // prior open_book_renderer call).
    service
        .open_if_needed(&db, book_id)
        .expect("lazy open from DB should succeed");

    assert!(service.is_open(book_id), "book should now be open");
    let toc = service.get_toc(book_id).expect("TOC should succeed");
    assert!(!toc.is_empty(), "TOC must be non-empty after lazy open");

    service.close_book(book_id);
    let _ = fs::remove_dir_all(&temp_dir);
}

/// A chapter query must also succeed via lazy open (get_book_chapter path).
#[tokio::test(flavor = "multi_thread")]
async fn chapter_available_after_lazy_open() {
    let (db, temp_dir) = create_temp_db();
    let file = temp_dir.join("book.txt");
    fs::write(&file, "Heading\n\nBody text here.").unwrap();
    let book_id = insert_book(&db, &file, "txt");

    let service = RenderingService::new(64);
    service.open_if_needed(&db, book_id).unwrap();

    let chapter = service
        .get_chapter(book_id, 0)
        .expect("chapter should load after lazy open");
    assert!(
        chapter.content.contains("Body text here"),
        "chapter content should contain the book text"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// An unknown book id must still surface BookNotFound.
#[tokio::test(flavor = "multi_thread")]
async fn unknown_book_id_still_returns_book_not_found() {
    let (db, temp_dir) = create_temp_db();
    let service = RenderingService::new(64);

    let err = service
        .open_if_needed(&db, 999_999)
        .expect_err("lazy open for an unknown book must fail");
    assert!(
        err.to_string().contains("not opened"),
        "expected BookNotFound, got: {}",
        err
    );

    let err = service
        .get_toc(999_999)
        .expect_err("TOC for an unknown book must fail");
    assert!(
        err.to_string().contains("not opened"),
        "expected BookNotFound, got: {}",
        err
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// open_if_needed must be a cheap no-op when the book is already open.
#[tokio::test(flavor = "multi_thread")]
async fn open_if_needed_is_noop_when_already_open() {
    let (db, temp_dir) = create_temp_db();
    let file = temp_dir.join("book.txt");
    fs::write(&file, "Heading\n\nBody text here.").unwrap();
    let book_id = insert_book(&db, &file, "txt");

    let service = RenderingService::new(64);
    service.open_book(book_id, file.to_str().unwrap(), "txt").unwrap();
    assert!(service.is_open(book_id));

    service
        .open_if_needed(&db, book_id)
        .expect("already-open book must be a no-op");
    assert!(service.is_open(book_id));

    let _ = fs::remove_dir_all(&temp_dir);
}
