use crate::services::online::provider::{ItemType, MetadataQuery};
use crate::services::online::worker::MetadataJob;
use crate::services::manga_metadata_service::parse_manga_title;
use tauri::State;

pub async fn enqueue_auto_metadata(
    db: &crate::db::Database,
    sender: &tokio::sync::mpsc::Sender<MetadataJob>,
    success_paths: &[String],
) {
    if success_paths.is_empty() {
        return;
    }
    
    let conn_res = db.get_connection();
    if conn_res.is_err() { return; }
    let conn = conn_res.unwrap();
    
    for path in success_paths {
        if let Ok(mut stmt) = conn.prepare("SELECT id, title, isbn, file_format, (SELECT name FROM book_authors ba JOIN authors a ON ba.author_id = a.id WHERE ba.book_id = books.id LIMIT 1) as author FROM books WHERE file_path = ?1") {
            if let Ok(mut rows) = stmt.query(rusqlite::params![path]) {
                if let Ok(Some(row)) = rows.next() {
                    let book_id: i64 = row.get(0).unwrap_or(0);
                    let title: String = row.get(1).unwrap_or_default();
                    let isbn: Option<String> = row.get(2).unwrap_or(None);
                    let file_format: String = row.get(3).unwrap_or_default();
                    let author: Option<String> = row.get(4).unwrap_or(None);
                    
                    if book_id > 0 {
                        let is_manga = matches!(file_format.to_lowercase().as_str(), "cbz" | "cbr");
                        let query = if is_manga {
                            MetadataQuery::Title(parse_manga_title(&title))
                        } else if let Some(isbn_val) = isbn {
                            MetadataQuery::Isbn(isbn_val)
                        } else {
                            MetadataQuery::TitleAuthor { title, author }
                        };
                        
                        let item_type = if is_manga { ItemType::Manga } else { ItemType::Book };
                        let job = MetadataJob {
                            item_id: book_id,
                            item_type,
                            query,
                            force_refresh: false,
                        };
                        let _ = sender.send(job).await;
                    }
                }
            }
        }
    }
}
