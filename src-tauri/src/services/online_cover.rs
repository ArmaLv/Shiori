//! Online cover lookup — Google Books API first, Open Library second.
//!
//! Used at import time (fire-and-forget background task) and on demand via the
//! `fetch_online_cover` command. Every failure degrades to `Ok(None)` with a
//! log line, so a missing/blocked network can never break import or cover
//! display — the caller falls back to the generated geometric cover.

use crate::db::Database;
use crate::error::{Result, ShioriError};
use crate::services::cover_service::CoverService;
use crate::services::library_service;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

/// Upper bound for a full lookup (Google Books + Open Library + image download).
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);
/// Per-request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

const GOOGLE_BOOKS_URL: &str = "https://www.googleapis.com/books/v1/volumes";
const OPENLIBRARY_SEARCH_URL: &str = "https://openlibrary.org/search.json";
const OPENLIBRARY_COVERS_URL: &str = "https://covers.openlibrary.org";

/// Books already attempted this app run — prevents hammering the APIs.
static ATTEMPTED_BOOK_IDS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();

/// Returns `true` (and marks the book) if this book id hasn't been attempted
/// yet this run. Used to guarantee at most one online lookup per book per run.
pub fn try_begin_online_cover_attempt(book_id: i64) -> bool {
    ATTEMPTED_BOOK_IDS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(book_id)
}

#[derive(Debug, Deserialize)]
struct GoogleBooksResponse {
    items: Option<Vec<GoogleBooksItem>>,
}

#[derive(Debug, Deserialize)]
struct GoogleBooksItem {
    volume_info: GoogleBooksVolumeInfo,
}

#[derive(Debug, Deserialize)]
struct GoogleBooksVolumeInfo {
    image_links: Option<GoogleBooksImageLinks>,
}

#[derive(Debug, Deserialize)]
struct GoogleBooksImageLinks {
    thumbnail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenLibrarySearchResponse {
    docs: Vec<OpenLibraryDoc>,
}

#[derive(Debug, Deserialize)]
struct OpenLibraryDoc {
    cover_i: Option<i64>,
}

/// HTTP client for cover lookups, with injectable endpoints (tests point them
/// at a dead local port so no real API is ever contacted).
pub struct OnlineCoverClient {
    client: Client,
    google_books_url: String,
    openlibrary_search_url: String,
    openlibrary_covers_url: String,
}

impl OnlineCoverClient {
    pub fn new() -> Result<Self> {
        Self::new_with_urls(
            GOOGLE_BOOKS_URL.to_string(),
            OPENLIBRARY_SEARCH_URL.to_string(),
            OPENLIBRARY_COVERS_URL.to_string(),
            REQUEST_TIMEOUT,
        )
    }

    /// Constructor with explicit endpoints and per-request timeout. Public for
    /// tests and alternative endpoints.
    pub fn new_with_urls(
        google_books_url: String,
        openlibrary_search_url: String,
        openlibrary_covers_url: String,
        request_timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(request_timeout)
            .user_agent("Shiori/2.1.3 (cover lookup)")
            .build()
            .map_err(|e| ShioriError::Other(format!("Failed to build HTTP client: {}", e)))?;
        Ok(Self {
            client,
            google_books_url,
            openlibrary_search_url,
            openlibrary_covers_url,
        })
    }

    /// Google Books first, Open Library second. Returns image bytes or None.
    pub async fn fetch_cover(
        &self,
        title: &str,
        author: Option<&str>,
        isbn: Option<&str>,
    ) -> Result<Option<Vec<u8>>> {
        if let Some(bytes) = self.google_books_cover(title, author).await? {
            return Ok(Some(bytes));
        }
        if let Some(bytes) = self.open_library_cover(title, author, isbn).await? {
            return Ok(Some(bytes));
        }
        Ok(None)
    }

    async fn google_books_cover(
        &self,
        title: &str,
        author: Option<&str>,
    ) -> Result<Option<Vec<u8>>> {
        let mut query = format!("intitle:{}", title);
        if let Some(author) = author {
            query.push_str(&format!("+inauthor:{}", author));
        }
        let url = format!(
            "{}?q={}&maxResults=1",
            self.google_books_url,
            urlencoding::encode(&query)
        );

        let body = match self.get_json(&url).await? {
            Some(b) => b,
            None => return Ok(None),
        };
        let parsed: GoogleBooksResponse = match serde_json::from_slice(&body) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("[online_cover] Google Books parse failed: {}", e);
                return Ok(None);
            }
        };
        let thumbnail = parsed
            .items
            .and_then(|items| items.into_iter().next())
            .and_then(|item| item.volume_info.image_links)
            .and_then(|links| links.thumbnail);
        let Some(thumbnail) = thumbnail else {
            log::debug!("[online_cover] Google Books: no thumbnail in response");
            return Ok(None);
        };

        // Google serves thumbnails over http:// at zoom=1 (small); upgrade to
        // https and request a larger zoom.
        let url = thumbnail
            .replace("http://", "https://")
            .replace("&zoom=1", "&zoom=3");
        self.download_image(&url).await
    }

    async fn open_library_cover(
        &self,
        title: &str,
        author: Option<&str>,
        isbn: Option<&str>,
    ) -> Result<Option<Vec<u8>>> {
        // ISBN is the most precise hit when we have it.
        if let Some(isbn) = isbn {
            let isbn = isbn.trim();
            if !isbn.is_empty() {
                let url = format!(
                    "{}/b/isbn/{}-L.jpg",
                    self.openlibrary_covers_url,
                    urlencoding::encode(isbn)
                );
                if let Some(bytes) = self.download_image(&url).await? {
                    return Ok(Some(bytes));
                }
            }
        }

        // Fall back to a title (+ author) search.
        let mut query = urlencoding::encode(title).into_owned();
        if let Some(author) = author {
            query.push('+');
            query.push_str(&urlencoding::encode(author));
        }
        let url = format!(
            "{}?q={}&limit=1&fields=cover_i",
            self.openlibrary_search_url, query
        );
        let body = match self.get_json(&url).await? {
            Some(b) => b,
            None => return Ok(None),
        };
        let parsed: OpenLibrarySearchResponse = match serde_json::from_slice(&body) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("[online_cover] Open Library parse failed: {}", e);
                return Ok(None);
            }
        };
        let Some(cover_i) = parsed.docs.into_iter().next().and_then(|d| d.cover_i) else {
            log::debug!("[online_cover] Open Library: no cover_i in response");
            return Ok(None);
        };
        let url = format!("{}/b/id/{}-L.jpg", self.openlibrary_covers_url, cover_i);
        self.download_image(&url).await
    }

    /// GET a JSON endpoint; None on any transport/status failure.
    async fn get_json(&self, url: &str) -> Result<Option<Vec<u8>>> {
        let resp = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                log::debug!("[online_cover] request failed ({}): {}", url, e);
                return Ok(None);
            }
        };
        if !resp.status().is_success() {
            log::debug!("[online_cover] HTTP {} for {}", resp.status(), url);
            return Ok(None);
        }
        match resp.bytes().await {
            Ok(b) => Ok(Some(b.to_vec())),
            Err(e) => {
                log::debug!("[online_cover] failed to read body ({}): {}", url, e);
                Ok(None)
            }
        }
    }

    /// Download an image, verifying the payload actually looks like an image.
    async fn download_image(&self, url: &str) -> Result<Option<Vec<u8>>> {
        let resp = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                log::debug!("[online_cover] image download failed ({}): {}", url, e);
                return Ok(None);
            }
        };
        if !resp.status().is_success() {
            log::debug!(
                "[online_cover] image download HTTP {} for {}",
                resp.status(),
                url
            );
            return Ok(None);
        }
        let bytes = match resp.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                log::debug!("[online_cover] failed to read image ({}): {}", url, e);
                return Ok(None);
            }
        };
        if bytes.len() < 512 || !looks_like_image(&bytes) {
            log::debug!(
                "[online_cover] downloaded payload is not an image ({} bytes)",
                bytes.len()
            );
            return Ok(None);
        }
        Ok(Some(bytes))
    }
}

fn looks_like_image(data: &[u8]) -> bool {
    data.starts_with(&[0xFF, 0xD8, 0xFF]) // jpeg
        || data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) // png
        || data.starts_with(b"GIF87a")
        || data.starts_with(b"GIF89a")
        || (data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP")
        || data.starts_with(b"BM") // bmp
}

/// Look up a cover online with a hard total timeout. Never errors on network
/// trouble — returns Ok(None) so callers fall through to the generated cover.
pub async fn fetch_online_cover(
    title: &str,
    author: Option<&str>,
    isbn: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let client = OnlineCoverClient::new()?;
    match tokio::time::timeout(LOOKUP_TIMEOUT, client.fetch_cover(title, author, isbn)).await {
        Ok(res) => res,
        Err(_) => {
            log::warn!("[online_cover] lookup timed out after {:?}", LOOKUP_TIMEOUT);
            Ok(None)
        }
    }
}

/// Fire-and-forget background lookup for a freshly imported book without an
/// embedded cover: fetch online → store via the CoverService → persist
/// `books.cover_path`. Failure is silent (log only) — the book keeps its
/// generated geometric cover. Skips gracefully when no Tokio runtime is
/// available (folder-watch threads, unit tests).
pub fn spawn_online_cover_lookup(
    db: Database,
    cover_service: Arc<CoverService>,
    book_id: i64,
    book_uuid: &str,
    title: String,
    authors: Vec<String>,
    isbn: Option<String>,
) {
    if tokio::runtime::Handle::try_current().is_err() {
        log::debug!(
            "[online_cover] no Tokio runtime — skipping background lookup for book {}",
            book_id
        );
        return;
    }
    if !try_begin_online_cover_attempt(book_id) {
        log::debug!("[online_cover] book {} already attempted this run", book_id);
        return;
    }
    let uuid = match Uuid::parse_str(book_uuid) {
        Ok(u) => u,
        Err(e) => {
            log::warn!("[online_cover] invalid book uuid {}: {}", book_uuid, e);
            return;
        }
    };

    tokio::spawn(async move {
        log::info!(
            "[online_cover] looking up cover online for book {} ({})",
            book_id,
            title
        );
        match fetch_online_cover(&title, authors.first().map(String::as_str), isbn.as_deref()).await
        {
            Ok(Some(bytes)) => match cover_service.store_cover_bytes(uuid, bytes).await {
                Ok(cover_set) => {
                    let path = cover_set.medium.to_string_lossy().to_string();
                    match library_service::update_book_cover_path(&db, book_id, Some(&path)) {
                        Ok(()) => log::info!(
                            "[online_cover] ✅ stored cover for book {} at {}",
                            book_id,
                            path
                        ),
                        Err(e) => log::warn!(
                            "[online_cover] failed to persist cover path for book {}: {}",
                            book_id,
                            e
                        ),
                    }
                }
                Err(e) => log::warn!(
                    "[online_cover] failed to store cover for book {}: {}",
                    book_id,
                    e
                ),
            },
            Ok(None) => {
                log::debug!("[online_cover] no cover found online for book {}", book_id)
            }
            Err(e) => log::warn!("[online_cover] lookup failed for book {}: {}", book_id, e),
        }
    });
}
