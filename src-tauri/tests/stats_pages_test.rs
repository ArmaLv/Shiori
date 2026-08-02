use shiori::{db::Database, services::reader_service::ReaderService};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn create_temp_db_and_covers() -> (Database, PathBuf, PathBuf) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir =
        std::env::temp_dir().join(format!("shiori_stats_test_{}_{}", std::process::id(), n));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("test.db");
    let covers_dir = temp_dir.join("covers");
    fs::create_dir_all(&covers_dir).unwrap();

    let db = Database::new(&db_path).unwrap();
    (db, db_path, covers_dir)
}

fn insert_book(conn: &rusqlite::Connection, title: &str, domain: &str) -> i64 {
    conn.execute(
        "INSERT INTO books (uuid, title, file_path, file_format, domain)
         VALUES (?1, ?2, ?3, 'pdf', ?4)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            title,
            format!("/tmp/{}_{}.pdf", title, uuid::Uuid::new_v4()),
            domain,
        ],
    )
    .unwrap();
    conn.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .unwrap()
}

fn insert_session(
    conn: &rusqlite::Connection,
    book_id: i64,
    pages_start: Option<i32>,
    pages_end: Option<i32>,
    duration_seconds: i64,
) {
    conn.execute(
        "INSERT INTO reading_sessions (id, book_id, started_at, ended_at, duration_seconds, pages_start, pages_end, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            book_id,
            chrono::Utc::now().to_rfc3339(),
            chrono::Utc::now().to_rfc3339(),
            duration_seconds,
            pages_start,
            pages_end,
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .unwrap();
}

#[test]
fn test_daily_stats_counts_pages_split_by_domain() {
    let (db, _db_path, _covers_dir) = create_temp_db_and_covers();
    let conn = db.get_connection().unwrap();

    let book_id = insert_book(&conn, "Some Book", "books");
    let manga_id = insert_book(&conn, "Some Manga", "manga");

    insert_session(&conn, book_id, Some(1), Some(15), 600);
    insert_session(&conn, manga_id, Some(1), Some(9), 300);
    // Negative delta must clamp to 0
    insert_session(&conn, book_id, Some(10), Some(3), 120);

    let stats = ReaderService::get_daily_reading_stats(&conn, 7).unwrap();
    assert_eq!(stats.len(), 1, "All sessions fall on today");
    assert_eq!(stats[0].books_count, 2);
    assert_eq!(stats[0].sessions_count, 3);
    assert_eq!(
        stats[0].book_pages_read, 14,
        "14 pages from book, negative delta clamped to 0"
    );
    assert_eq!(stats[0].manga_pages_read, 8);
}

#[test]
fn test_start_session_backfills_pages_start_from_progress() {
    let (db, _db_path, _covers_dir) = create_temp_db_and_covers();
    let conn = db.get_connection().unwrap();

    let book_id = insert_book(&conn, "Progress Book", "books");

    // No progress yet -> pages_start stays None
    let session = ReaderService::start_reading_session(&conn, book_id, None).unwrap();
    assert_eq!(session.pages_start, None);

    // Save progress at page 7, then start a new session without pages_start
    ReaderService::save_reading_progress(&conn, book_id, "loc-1", 0.5, Some(7), Some(100), None)
        .unwrap();

    let session = ReaderService::start_reading_session(&conn, book_id, None).unwrap();
    assert_eq!(session.pages_start, Some(7));

    // Explicit pages_start is respected over backfill
    let session = ReaderService::start_reading_session(&conn, book_id, Some(3)).unwrap();
    assert_eq!(session.pages_start, Some(3));
}

#[test]
fn test_end_session_backfills_pages_end_from_progress() {
    let (db, _db_path, _covers_dir) = create_temp_db_and_covers();
    let conn = db.get_connection().unwrap();

    let book_id = insert_book(&conn, "End Book", "books");
    let session = ReaderService::start_reading_session(&conn, book_id, Some(1)).unwrap();

    // No progress yet -> pages_end stays None
    ReaderService::end_reading_session(&conn, &session.id, None).unwrap();
    let pages_end: Option<i32> = conn
        .query_row(
            "SELECT pages_end FROM reading_sessions WHERE id = ?1",
            rusqlite::params![session.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pages_end, None);

    // Save progress at page 7, then end without pages_end
    ReaderService::save_reading_progress(&conn, book_id, "loc-2", 0.6, Some(7), Some(100), None)
        .unwrap();

    let session = ReaderService::start_reading_session(&conn, book_id, Some(1)).unwrap();
    ReaderService::end_reading_session(&conn, &session.id, None).unwrap();
    let pages_end: Option<i32> = conn
        .query_row(
            "SELECT pages_end FROM reading_sessions WHERE id = ?1",
            rusqlite::params![session.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pages_end, Some(7));
}
