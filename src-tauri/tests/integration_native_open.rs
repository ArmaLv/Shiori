//! Integration tests for the native multi-format open pipeline (S1).
//!
//! Verifies the PLAN contract:
//! - `open_book_for_reading` resolves to the ORIGINAL file path for every
//!   native-readable format and never rewrites `books.file_path`/`file_format`.
//! - The explicit `convert_book` flow is non-destructive: DB row untouched,
//!   original file untouched, output lands in a temp cache path.
//!
//! Commands themselves are thin wrappers over `services::reader_service`
//! helpers (Tauri `State`/`WebviewWindow` are not constructible in tests), so
//! the helpers are what gets exercised here.

use shiori::db::Database;
use shiori::services::reader_service::{self, BookOpenPath};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static BOOK_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn create_temp_db_and_covers() -> (Database, PathBuf, PathBuf) {
    let temp_dir = std::env::temp_dir().join(format!(
        "shiori_native_test_{}_{}",
        std::process::id(),
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("test.db");
    let covers_dir = temp_dir.join("covers");
    fs::create_dir_all(&covers_dir).unwrap();

    let db = Database::new(&db_path).unwrap();
    (db, db_path, covers_dir)
}

fn insert_book(db: &Database, path: &std::path::Path, format: &str) -> i64 {
    let conn = db.get_connection().unwrap();
    let uuid = format!(
        "test-{}-{}",
        std::process::id(),
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    conn.execute(
        "INSERT INTO books (uuid, title, file_path, file_format, file_size) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            uuid,
            "Test Book",
            path.to_string_lossy().to_string(),
            format,
            1234
        ],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn book_row(db: &Database, book_id: i64) -> (String, String) {
    let conn = db.get_connection().unwrap();
    conn.query_row(
        "SELECT file_path, file_format FROM books WHERE id = ?1",
        rusqlite::params![book_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap()
}

/// Every native format must resolve to the ORIGINAL path — no conversion,
/// no DB rewrite (PLAN contract for open_book_for_reading).
#[test]
fn open_book_for_reading_returns_original_path_for_all_native_formats() {
    let (db, _db_path, _covers) = create_temp_db_and_covers();
    let dir = std::env::temp_dir().join(format!(
        "shiori_native_books_{}",
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();

    for format in [
        "pdf", "mobi", "docx", "fb2", "txt", "html", "md", "azw3", "cbz", "cbr", "epub",
    ] {
        let file = dir.join(format!("book_{}.{}", format, format));
        fs::write(&file, "dummy content for native open test").unwrap();

        let book_id = insert_book(&db, &file, format);

        let decision = reader_service::resolve_book_open_path(&db, book_id)
            .unwrap_or_else(|e| panic!("resolve failed for {}: {}", format, e));

        let BookOpenPath::Native(path) = decision else {
            panic!("{} should resolve natively, got NeedsConversion", format);
        };
        assert_eq!(
            path,
            file.to_string_lossy().to_string(),
            "{} must resolve to the original file path",
            format
        );

        // DB row must be untouched: same path, same format
        let (db_path, db_format) = book_row(&db, book_id);
        assert_eq!(db_path, file.to_string_lossy().to_string());
        assert_eq!(db_format, format);
    }

    let _ = fs::remove_dir_all(&dir);
}

/// Formats without a native reader still route to the conversion fallback.
#[test]
fn unknown_format_still_routes_to_conversion_fallback() {
    let (db, _db_path, _covers) = create_temp_db_and_covers();
    let dir = std::env::temp_dir().join(format!(
        "shiori_native_books_{}",
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();

    let file = dir.join("book.weird");
    fs::write(&file, "data").unwrap();
    let book_id = insert_book(&db, &file, "weird");

    let decision = reader_service::resolve_book_open_path(&db, book_id).unwrap();
    assert!(
        matches!(decision, BookOpenPath::NeedsConversion { .. }),
        "unknown format should fall through to conversion"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// convert_book on an unsupported source (cbz) must fail cleanly and leave
/// the DB row and the original file completely untouched.
#[tokio::test]
async fn convert_book_cbz_fails_without_touching_db_or_original() {
    let (db, _db_path, _covers) = create_temp_db_and_covers();
    let dir = std::env::temp_dir().join(format!(
        "shiori_native_books_{}",
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();

    let file = dir.join("book.cbz");
    fs::write(&file, "dummy cbz data").unwrap();
    let book_id = insert_book(&db, &file, "cbz");

    let result = reader_service::convert_book_to_epub(&db, book_id, None).await;
    assert!(result.is_err(), "cbz → epub is not in the engine matrix");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not supported"),
        "expected an unsupported-conversion error, got: {}",
        err
    );

    // DB row untouched
    let (db_path, db_format) = book_row(&db, book_id);
    assert_eq!(db_path, file.to_string_lossy().to_string());
    assert_eq!(db_format, "cbz");

    // Original file still exists
    assert!(file.exists(), "original file must never be deleted");

    let _ = fs::remove_dir_all(&dir);
}

/// convert_book on a supported source (txt) produces a real .epub in a temp
/// cache dir while leaving the DB row and the original file untouched.
#[tokio::test]
async fn convert_book_txt_produces_epub_without_touching_db_or_original() {
    let (db, _db_path, _covers) = create_temp_db_and_covers();
    let dir = std::env::temp_dir().join(format!(
        "shiori_native_books_{}",
        BOOK_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir_all(&dir).unwrap();

    let file = dir.join("book.txt");
    let original_content = "The Title\n\nThis is a perfectly ordinary text document with enough \
         words to survive a conversion round trip. It has several sentences \
         and a second paragraph to make the output look like a real book.";
    fs::write(&file, original_content).unwrap();
    let book_id = insert_book(&db, &file, "txt");

    let result = reader_service::convert_book_to_epub(&db, book_id, None).await;
    let converted = result.unwrap_or_else(|e| panic!("txt → epub conversion failed: {}", e));

    assert_eq!(converted.new_format, "epub");
    assert!(
        converted.new_path.ends_with(".epub"),
        "output must be an epub, got: {}",
        converted.new_path
    );

    let out = std::path::Path::new(&converted.new_path);
    assert!(
        out.exists(),
        "converted file must exist at {}",
        converted.new_path
    );

    // Output lives in the temp cache dir, NOT next to the original
    assert!(
        !out.starts_with(&dir),
        "converted file must not be written into the library folder"
    );

    // DB row untouched
    let (db_path, db_format) = book_row(&db, book_id);
    assert_eq!(db_path, file.to_string_lossy().to_string());
    assert_eq!(db_format, "txt");

    // Original file still exists, unchanged
    assert!(file.exists(), "original file must never be deleted");
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        original_content,
        "original file content must be unchanged"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// book_needs_conversion semantics: false for every native format, true otherwise.
#[test]
fn book_needs_conversion_matches_native_format_set() {
    for format in [
        "epub", "pdf", "mobi", "azw3", "docx", "fb2", "txt", "html", "htm", "md", "cbz", "cbr",
    ] {
        assert!(
            reader_service::is_native_readable_format(format),
            "{} should be native-readable",
            format
        );
    }
    assert!(!reader_service::is_native_readable_format("weird"));
    assert!(!reader_service::is_native_readable_format("audio"));
}
