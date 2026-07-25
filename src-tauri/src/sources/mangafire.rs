use async_trait::async_trait;
use dashmap::DashMap;
use serde::Deserialize;
#[cfg(target_os = "android")]
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;



use crate::cloudflare::client::CfClient;
use crate::error::{Result, ShioriError};
use crate::sources::{Chapter, ContentType, Page, SearchResult, Source, SourceMeta};

const BASE_URL: &str = "https://mangafire.to";

pub struct MangaFireSource {
    cf_client: RwLock<Option<Arc<CfClient>>>,
    app_handle: RwLock<Option<tauri::AppHandle>>,
    /// Pending XHR requests waiting for frontend bridge response (Android only)
    pending_requests: Arc<DashMap<String, tokio::sync::oneshot::Sender<std::result::Result<String, String>>>>,
}

impl MangaFireSource {
    pub fn new() -> Self {
        Self {
            cf_client: RwLock::new(None),
            app_handle: RwLock::new(None),
            pending_requests: Arc::new(DashMap::new()),
        }
    }

    pub async fn set_cf_client(&self, cf: Arc<CfClient>, app_handle: tauri::AppHandle) {
        *self.cf_client.write().await = Some(cf);
        *self.app_handle.write().await = Some(app_handle);
    }

    /// Called by the Tauri command `mangafire_xhr_response` to resolve a pending request.
    pub fn resolve_xhr(&self, id: &str, body: Option<String>, error: Option<String>) {
        if let Some((_, tx)) = self.pending_requests.remove(id) {
            let result = match (body, error) {
                (Some(b), _) => Ok(b),
                (_, Some(e)) => Err(e),
                _ => Err("Empty response from bridge".to_string()),
            };
            let _ = tx.send(result);
        }
    }

    async fn wait_for_init(&self) -> Result<()> {
        for _ in 0..50 {
            {
                let cf_ready = self.cf_client.read().await.is_some();
                let app_ready = self.app_handle.read().await.is_some();
                if cf_ready && app_ready {
                    return Ok(());
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Err(ShioriError::Other("MangaFire source client not initialized (timeout)".into()))
    }

    async fn evaluate_js_on_site(&self, js_script: &str) -> Result<String> {
        self.wait_for_init().await?;
        let guard = self.app_handle.read().await;
        if let Some(app) = guard.as_ref() {
            let app = app.clone();
            let window_label = format!("mf-rpc-{}", uuid::Uuid::new_v4().simple());
            let (tx, rx) = tokio::sync::oneshot::channel();
            let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));
            let html_buffer = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

            let js = format!(
                r#"(async () => {{
                    try {{
                        if (window.top !== window.self) return;
                        if (document.readyState === 'loading') {{
                            await new Promise(resolve => document.addEventListener('DOMContentLoaded', resolve));
                        }}
                        while (true) {{
                            const title = document.title.toLowerCase();
                            if (title.includes('just a moment') || title.includes('cloudflare') || title.includes('attention required')) {{
                                await new Promise(resolve => setTimeout(resolve, 1000));
                                continue;
                            }}
                            break;
                        }}
                        
                        let attempts = 0;
                        while (typeof window.extendClient === 'undefined' && attempts < 25) {{
                            await new Promise(r => setTimeout(r, 400));
                            attempts++;
                        }}
                        if (typeof window.extendClient === 'undefined') throw new Error("extendClient not found");

                        if (!window.myAxios) {{
                            let requestInterceptor = null;
                            window.myAxios = {{
                                defaults: {{ baseURL: '/', headers: {{}} }},
                                interceptors: {{
                                    request: {{
                                        use: (fn) => {{ requestInterceptor = fn; }}
                                    }}
                                }},
                                get: async (url, config = {{}}) => {{
                                    let reqConfig = {{
                                        url,
                                        method: 'get',
                                        headers: {{
                                            'Accept': 'application/json, text/javascript, */*; q=0.01',
                                            'X-Requested-With': 'XMLHttpRequest',
                                            ...(config.headers || {{}})
                                        }},
                                        params: config.params || {{}}
                                    }};
                                    if (requestInterceptor) {{
                                        reqConfig = await requestInterceptor(reqConfig) || reqConfig;
                                    }}
                                    let fullUrl = reqConfig.url;
                                    if (reqConfig.params && Object.keys(reqConfig.params).length > 0) {{
                                        const query = new URLSearchParams(reqConfig.params).toString();
                                        fullUrl += (fullUrl.includes('?') ? '&' : '?') + query;
                                    }}
                                    const resp = await fetch(fullUrl, {{
                                        method: 'GET',
                                        headers: reqConfig.headers,
                                        credentials: 'include'
                                    }});
                                    const data = await resp.json();
                                    return {{ data }};
                                }}
                            }};
                            window.extendClient(window.myAxios);
                        }}

                        const raw_result = await (async () => {{ {} }})();
                        const result = (typeof raw_result === 'string') ? raw_result : JSON.stringify(raw_result);
                        const chunkSize = 512;
                        window.__CHUNK_ACK = true;
                        for (let i = 0; i < result.length; i += chunkSize) {{
                            const chunk = result.slice(i, i + chunkSize);
                            window.__CHUNK_ACK = false;
                            document.title = 'SHIORI_CHUNK|' + encodeURIComponent(chunk);
                            while (!window.__CHUNK_ACK) {{
                                await new Promise(r => setTimeout(r, 10));
                            }}
                        }}
                        document.title = 'SHIORI_DONE|';
                    }} catch (e) {{
                        document.title = 'SHIORI_ERROR|' + e.message;
                    }}
                }})();"#,
                js_script
            );

            let tx_clone = std::sync::Arc::clone(&tx);
            let app_clone = app.clone();
            let window_label_clone = window_label.clone();
            let html_buffer_clone = std::sync::Arc::clone(&html_buffer);

            use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

            let _window = WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::External("https://mangafire.to/filter".parse().unwrap()))
                .visible(false)
                .initialization_script(&js)
                .on_document_title_changed(move |window, title| {
                    if title.starts_with("SHIORI_CHUNK|") {
                        if let Ok(mut buf) = html_buffer_clone.lock() {
                            let raw = title.trim_start_matches("SHIORI_CHUNK|");
                            let decoded = urlencoding::decode(raw).unwrap_or(std::borrow::Cow::Borrowed(raw));
                            buf.push_str(&decoded);
                        }
                        let _ = window.eval("window.__CHUNK_ACK = true;");
                    } else if title.starts_with("SHIORI_DONE|") {
                        if let Ok(mut lock) = tx_clone.lock() {
                            if let Some(sender) = lock.take() {
                                let buf = html_buffer_clone.lock().unwrap().clone();
                                let _ = sender.send(Ok(buf));
                            }
                        }
                        let w_label = window_label_clone.clone();
                        let a = app_clone.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(w) = a.get_webview_window(&w_label) {
                                let _ = w.close();
                            }
                        });
                    } else if title.starts_with("SHIORI_ERROR|") {
                        if let Ok(mut lock) = tx_clone.lock() {
                            if let Some(sender) = lock.take() {
                                let err = title.trim_start_matches("SHIORI_ERROR|").to_string();
                                let _ = sender.send(Err(err));
                            }
                        }
                        let w_label = window_label_clone.clone();
                        let a = app_clone.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Some(w) = a.get_webview_window(&w_label) {
                                let _ = w.close();
                            }
                        });
                    }
                })
                .build()
                .map_err(|e| ShioriError::Other(format!("Failed to build rpc webview: {}", e)))?;

            let result = match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
                Ok(Ok(Ok(res))) => res,
                Ok(Ok(Err(err))) => {
                    return Err(ShioriError::Other(format!("MangaFire RPC JS error: {}", err)));
                }
                _ => {
                    let w_label = window_label.clone();
                    let a = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(w) = a.get_webview_window(&w_label) {
                            let _ = w.close();
                        }
                    });
                    return Err(ShioriError::Other("MangaFire RPC timed out".to_string()));
                }
            };

            return Ok(result);
        }
        Err(ShioriError::Other("Browser RPC not initialized for MangaFire".into()))
    }

    /// Fetch a MangaFire API path via the frontend WebView bridge (Android).
    /// The main WebView already has CF cookies so the XHR will succeed.
    #[cfg(target_os = "android")]
    async fn fetch_rpc_via_bridge(&self, url: &str) -> Result<String> {
        self.wait_for_init().await?;
        let guard = self.app_handle.read().await;
        let app = guard.as_ref().ok_or_else(|| ShioriError::Other("MangaFire app handle not ready".into()))?.clone();
        drop(guard);

        let full_url = if url.starts_with('/') {
            format!("{}{}", BASE_URL, url)
        } else {
            url.to_string()
        };

        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.insert(request_id.clone(), tx);

        #[derive(Serialize)]
        struct XhrRequestPayload {
            id: String,
            url: String,
        }

        use tauri::Emitter;
        if let Err(e) = app.emit("mf-xhr-request", XhrRequestPayload {
            id: request_id.clone(),
            url: full_url,
        }) {
            self.pending_requests.remove(&request_id);
            return Err(ShioriError::Other(format!("Failed to emit mf-xhr-request: {e}")));
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(Ok(body))) => Ok(body),
            Ok(Ok(Err(e))) => Err(ShioriError::Other(format!("MangaFire XHR bridge error: {e}"))),
            Ok(Err(_)) => {
                self.pending_requests.remove(&request_id);
                Err(ShioriError::Other("MangaFire XHR bridge channel dropped".into()))
            }
            Err(_) => {
                self.pending_requests.remove(&request_id);
                Err(ShioriError::Other("MangaFire XHR bridge timed out (30s)".into()))
            }
        }
    }

    async fn fetch_rpc(&self, url: &str) -> Result<String> {
        // On Android, WebviewWindowBuilder navigates the main (only) WebView,
        // which would take the entire app to mangafire.to. Use the frontend bridge instead:
        // Rust emits an event to the WebView, the WebView does fetch() with its own CF cookies,
        // then calls back via mangafire_xhr_response Tauri command.
        #[cfg(target_os = "android")]
        {
            return self.fetch_rpc_via_bridge(url).await;
        }

        #[cfg(not(target_os = "android"))]
        {
            let js = format!(r#"
                const [path, queryString] = '{}'.split('?');
                const queryParams = {{}};
                if (queryString) {{
                    const searchParams = new URLSearchParams(queryString);
                    for (const [key, value] of searchParams.entries()) {{
                        queryParams[key] = value;
                    }}
                }}
                let res = await window.myAxios.get(path, {{ params: queryParams }});
                return res.data;
            "#, url);
            self.evaluate_js_on_site(&js).await
        }
    }
}

#[derive(Debug, Deserialize)]
struct MfSearchResponse {
    items: Vec<MfSearchItem>,
}

#[derive(Debug, Deserialize)]
struct MfSearchItem {
    hid: String,
    slug: String,
    title: String,
    poster: Option<MfPoster>,
}

#[derive(Debug, Deserialize)]
struct MfPoster {
    large: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MfChaptersResponse {
    items: Vec<MfChapterItem>,
    meta: Option<MfMeta>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct MfMeta {
    last_page: u32,
}

#[derive(Debug, Deserialize)]
struct MfChapterItem {
    id: u64,
    number: f32,
    name: String,
    language: String,
}

#[derive(Debug, Deserialize)]
struct MfPageResponse {
    data: MfPageData,
}

#[derive(Debug, Deserialize)]
struct MfPageData {
    pages: Vec<MfPageItem>,
}

#[derive(Debug, Deserialize)]
struct MfPageItem {
    url: String,
}

#[async_trait]
impl Source for MangaFireSource {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn meta(&self) -> SourceMeta {
        SourceMeta {
            id: "mangafire".to_string(),
            name: "MangaFire".to_string(),
            base_url: BASE_URL.to_string(),
            version: "1.0.0".to_string(),
            content_type: ContentType::Manga,
            supports_search: true,
            supports_download: true,
            requires_api_key: false,
            nsfw: false,
        }
    }

    async fn search(&self, query: &str, _page: u32) -> Result<Vec<SearchResult>> {
        // URL encode the query
        let encoded_query = urlencoding::encode(query);
        let url = format!("/api/titles?keyword={}&page=1&limit=50", encoded_query);

        let json_str = self.fetch_rpc(&url).await?;
        let res: MfSearchResponse = serde_json::from_str(&json_str)
            .map_err(|e| ShioriError::Other(format!("Failed to parse MangaFire search JSON: {} - raw: {}", e, json_str)))?;

        let mut results = Vec::new();
        for item in res.items {
            let cover_url = item.poster.and_then(|p| p.large);
            // We encode hid and slug in the ID so we can use it in get_chapters
            let id = format!("{}|{}", item.hid, item.slug);

            results.push(SearchResult {
                id,
                title: item.title,
                cover_url,
                description: None,
                source_id: "mangafire".to_string(),
                extra: HashMap::new(),
            });
        }

        Ok(results)
    }

    async fn browse(
        &self,
        mode: &str,
        page: u32,
        _limit: u32,
        _genres: Option<Vec<String>>,
        _types: Option<Vec<String>>,
    ) -> Result<Vec<SearchResult>> {
        let mut base_url = match mode {
            "popular" => format!("/api/titles?order[chapter_updated_at]=desc&hot=1&page={}&limit=30", page),
            "latest" | "recent" => format!("/api/titles?order[chapter_updated_at]=desc&page={}&limit=30", page),
            _ => format!("/api/titles?order[chapter_updated_at]=desc&page={}&limit=30", page),
        };

        if let Some(genres) = _genres {
            if !genres.is_empty() {
                let slugs: Vec<String> = genres.into_iter().map(|g| g.to_lowercase().replace(" ", "-")).collect();
                base_url.push_str(&format!("&genre={}", slugs.join(",")));
            }
        }

        if let Some(types) = _types {
            if !types.is_empty() {
                let slugs: Vec<String> = types.into_iter().map(|t| t.to_lowercase().replace(" ", "-")).collect();
                base_url.push_str(&format!("&type={}", slugs.join(",")));
            }
        }

        let url = base_url;

        let json_str = self.fetch_rpc(&url).await?;
        let res: MfSearchResponse = serde_json::from_str(&json_str)
            .map_err(|e| ShioriError::Other(format!("Failed to parse MangaFire browse JSON: {} - raw: {}", e, json_str)))?;

        let mut results = Vec::new();
        for item in res.items {
            let cover_url = item.poster.and_then(|p| p.large);
            let id = format!("{}|{}", item.hid, item.slug);

            results.push(SearchResult {
                id,
                title: item.title,
                cover_url,
                description: None,
                source_id: "mangafire".to_string(),
                extra: HashMap::new(),
            });
        }

        Ok(results)
    }

    async fn get_chapters(&self, content_id: &str) -> Result<Vec<Chapter>> {
        let parts: Vec<&str> = content_id.split('|').collect();
        if parts.len() != 2 {
            return Err(ShioriError::Other("Invalid MangaFire content ID".to_string()));
        }
        let hid = parts[0];
        let _slug = parts[1];

        let mut all_items = Vec::new();
        let mut current_page = 1;
        let mut last_page = 1;

        loop {
            let url = format!("/api/titles/{}/chapters?language=en&sort=number&order=desc&page={}&limit=200", hid, current_page);
            let json_str = match self.fetch_rpc(&url).await {
                Ok(s) => s,
                Err(e) => {
                    if current_page == 1 {
                        return Err(e);
                    }
                    break;
                }
            };
            
            let res: MfChaptersResponse = serde_json::from_str(&json_str)
                .map_err(|e| ShioriError::Other(format!("Failed to parse MangaFire chapters JSON: {}", e)))?;
                
            all_items.extend(res.items);
            
            if let Some(meta) = res.meta {
                last_page = meta.last_page;
            }
            
            if current_page >= last_page {
                break;
            }
            current_page += 1;
        }

        let mut chapters = Vec::new();
        for item in all_items {
            if item.language != "en" {
                continue;
            }

            let chap_id = item.id.to_string();
            let title = if item.name.trim().is_empty() {
                format!("Chapter {}", item.number)
            } else {
                item.name
            };

            chapters.push(Chapter {
                id: chap_id,
                title,
                number: item.number,
                volume: None,
                uploaded_at: None, // Could parse if needed, but not critical
                source_id: "mangafire".to_string(),
                content_id: content_id.to_string(),
            });
        }

        // Return deduplicated chapters (sometimes multiple groups upload same number)
        let mut unique_chapters: Vec<Chapter> = Vec::new();
        let mut seen_numbers = std::collections::HashSet::new();
        for ch in chapters {
            let num_str = ch.number.to_string();
            if !seen_numbers.contains(&num_str) {
                seen_numbers.insert(num_str);
                unique_chapters.push(ch);
            }
        }

        Ok(unique_chapters)
    }

    async fn get_pages(&self, chapter_id: &str) -> Result<Vec<Page>> {
        // chapter_id is just the id (e.g., 7285952)
        let url = format!("/api/chapters/{}", chapter_id);

        let json_str = self.fetch_rpc(&url).await?;
        let res: MfPageResponse = serde_json::from_str(&json_str)
            .map_err(|e| ShioriError::Other(format!("Failed to parse MangaFire pages JSON: {} - raw: {}", e, json_str)))?;

        let mut pages = Vec::new();
        for (i, p) in res.data.pages.into_iter().enumerate() {
            pages.push(Page {
                index: i as u32,
                url: p.url,
            });
        }

        Ok(pages)
    }
}
