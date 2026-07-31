/// Translation & Dictionary Service
///
/// Provides dictionary lookups (Free Dictionary API) and text translation
/// (MyMemory API with Lingva fallback). All APIs are free and require no keys.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;

use crate::error::{Result, ShioriError};

static TRANSLATION_CACHE: OnceLock<Mutex<HashMap<String, TranslationResult>>> = OnceLock::new();

fn get_translation_cache() -> &'static Mutex<HashMap<String, TranslationResult>> {
    TRANSLATION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
// PUBLIC TYPES
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryResult {
    pub word: String,
    pub phonetic: Option<String>,
    pub audio_url: Option<String>,
    pub meanings: Vec<DictionaryMeaning>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryMeaning {
    pub part_of_speech: String,
    pub definitions: Vec<DictionaryDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryDefinition {
    pub definition: String,
    pub example: Option<String>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub provider: String,
}

// ═══════════════════════════════════════════════════════════════
// API RESPONSE TYPES (internal)
// ═══════════════════════════════════════════════════════════════

// --- Free Dictionary API (dictionaryapi.dev) ---

#[derive(Debug, Deserialize)]
struct FreeDictEntry {
    word: String,
    phonetic: Option<String>,
    phonetics: Option<Vec<FreeDictPhonetic>>,
    meanings: Vec<FreeDictMeaning>,
    #[serde(rename = "sourceUrls")]
    source_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct FreeDictPhonetic {
    text: Option<String>,
    audio: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FreeDictMeaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: String,
    definitions: Vec<FreeDictDefinition>,
}

#[derive(Debug, Deserialize)]
struct FreeDictDefinition {
    definition: String,
    example: Option<String>,
    #[serde(default)]
    synonyms: Vec<String>,
    #[serde(default)]
    antonyms: Vec<String>,
}

// --- Wiktionary REST API (fallback dictionary) ---
// Response is a map of language code -> list of part-of-speech entries.
// Definitions and examples contain HTML markup that must be stripped.

#[derive(Debug, Deserialize)]
struct WiktEntry {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    #[serde(default)]
    definitions: Vec<WiktDefinition>,
}

#[derive(Debug, Deserialize)]
struct WiktDefinition {
    #[serde(default)]
    definition: String,
    #[serde(default)]
    examples: Vec<String>,
}

// --- MyMemory Translation API ---

#[derive(Debug, Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: MyMemoryData,
    #[serde(rename = "responseStatus")]
    response_status: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct MyMemoryData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

// --- Lingva Translate API (fallback) ---

#[derive(Debug, Deserialize)]
struct LingvaResponse {
    translation: String,
}

// ═══════════════════════════════════════════════════════════════
// SERVICE IMPLEMENTATION
// ═══════════════════════════════════════════════════════════════

fn build_client() -> std::result::Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Shiori/0.1.0")
        .build()
}

/// Look up a word, trying the Free Dictionary API first and falling back to
/// Wiktionary if the primary provider is unavailable or has no entry.
///
/// Both providers are free and keyless. If both fail, the primary provider's
/// error is surfaced (it produces the friendlier user-facing messages).
pub async fn dictionary_lookup(word: &str, lang: &str) -> Result<DictionaryResult> {
    match free_dictionary_lookup(word, lang).await {
        Ok(result) => Ok(result),
        Err(primary_err) => match wiktionary_lookup(word, lang).await {
            Ok(result) => Ok(result),
            // Both failed: surface the primary error (better wording than the
            // fallback's, and it distinguishes "unavailable" from "not found").
            Err(_fallback_err) => Err(primary_err),
        },
    }
}

/// Primary provider: the Free Dictionary API (dictionaryapi.dev).
/// Returns structured definitions, phonetics, and examples.
/// Supports English primarily; other languages via lang code.
async fn free_dictionary_lookup(word: &str, lang: &str) -> Result<DictionaryResult> {
    let client =
        build_client().map_err(|e| ShioriError::Other(format!("HTTP client error: {}", e)))?;

    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/{}/{}",
        urlencoding::encode(lang),
        urlencoding::encode(word)
    );

    // The Free Dictionary API is a free, community-run service with no SLA and
    // frequently returns transient 5xx errors (502/503/504) or times out under
    // load. Retry a few times with a short backoff before giving up.
    const MAX_ATTEMPTS: u32 = 3;
    let mut response = None;
    let mut last_transient_error: Option<String> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match client.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                // 502/503/504 are transient upstream failures worth retrying.
                if matches!(status.as_u16(), 502 | 503 | 504) {
                    last_transient_error =
                        Some(format!("Dictionary service returned status {}", status));
                } else {
                    response = Some(resp);
                    break;
                }
            }
            Err(e) => {
                // Network/timeout errors are also transient.
                last_transient_error = Some(format!("Dictionary request failed: {}", e));
            }
        }

        // Back off before the next attempt (skip the wait after the final try).
        if attempt < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(400 * attempt as u64)).await;
        }
    }

    let response = match response {
        Some(resp) => resp,
        None => {
            return Err(ShioriError::Other(format!(
                "Dictionary service is temporarily unavailable. Please try again in a moment. ({})",
                last_transient_error.unwrap_or_else(|| "no response".to_string())
            )));
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(ShioriError::Other(format!(
                "No definition found for \"{}\"",
                word
            )));
        }
        return Err(ShioriError::Other(format!(
            "Dictionary API returned status {}",
            status
        )));
    }

    let entries: Vec<FreeDictEntry> = response
        .json()
        .await
        .map_err(|e| ShioriError::Other(format!("Failed to parse dictionary response: {}", e)))?;

    let entry = entries
        .into_iter()
        .next()
        .ok_or_else(|| ShioriError::Other("Empty dictionary response".to_string()))?;

    // Extract best phonetic and audio
    let phonetic = entry.phonetic.clone().or_else(|| {
        entry
            .phonetics
            .as_ref()
            .and_then(|ps| ps.iter().find_map(|p| p.text.clone()))
    });

    let audio_url = entry.phonetics.as_ref().and_then(|ps| {
        ps.iter()
            .find_map(|p| p.audio.as_ref().filter(|a| !a.is_empty()).cloned())
    });

    let source_url = entry.source_urls.and_then(|urls| urls.into_iter().next());

    let meanings = entry
        .meanings
        .into_iter()
        .map(|m| DictionaryMeaning {
            part_of_speech: m.part_of_speech,
            definitions: m
                .definitions
                .into_iter()
                .take(3) // Limit to 3 definitions per part of speech
                .map(|d| DictionaryDefinition {
                    definition: d.definition,
                    example: d.example,
                    synonyms: d.synonyms.into_iter().take(5).collect(),
                    antonyms: d.antonyms.into_iter().take(5).collect(),
                })
                .collect(),
        })
        .collect();

    Ok(DictionaryResult {
        word: entry.word,
        phonetic,
        audio_url,
        meanings,
        source_url,
    })
}

/// Fallback provider: Wiktionary's REST API.
///
/// Used when the Free Dictionary API is unavailable or has no entry. Returns
/// definitions grouped by part of speech for the requested language. Wiktionary
/// definitions come as HTML, so tags are stripped and entities decoded.
async fn wiktionary_lookup(word: &str, lang: &str) -> Result<DictionaryResult> {
    let client =
        build_client().map_err(|e| ShioriError::Other(format!("HTTP client error: {}", e)))?;

    // Wiktionary keys definitions by language code (e.g. "en", "fr").
    let lang_key = lang.to_lowercase();

    let url = format!(
        "https://en.wiktionary.org/api/rest_v1/page/definition/{}",
        urlencoding::encode(word)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ShioriError::Other(format!("Wiktionary request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(ShioriError::Other(format!(
                "No definition found for \"{}\"",
                word
            )));
        }
        return Err(ShioriError::Other(format!(
            "Wiktionary API returned status {}",
            status
        )));
    }

    let by_lang: HashMap<String, Vec<WiktEntry>> = response
        .json()
        .await
        .map_err(|e| ShioriError::Other(format!("Failed to parse Wiktionary response: {}", e)))?;

    // Prefer the requested language; otherwise fall back to English.
    let entries = by_lang
        .get(&lang_key)
        .or_else(|| by_lang.get("en"))
        .ok_or_else(|| ShioriError::Other(format!("No definition found for \"{}\"", word)))?;

    let strip = crate::conversion::utils::strip_html_tags;

    let meanings: Vec<DictionaryMeaning> = entries
        .iter()
        .filter_map(|entry| {
            let definitions: Vec<DictionaryDefinition> = entry
                .definitions
                .iter()
                .filter_map(|d| {
                    let text = strip(&d.definition).trim().to_string();
                    if text.is_empty() {
                        return None;
                    }
                    let example = d
                        .examples
                        .iter()
                        .map(|e| strip(e).trim().to_string())
                        .find(|e| !e.is_empty());
                    Some(DictionaryDefinition {
                        definition: text,
                        example,
                        synonyms: Vec::new(),
                        antonyms: Vec::new(),
                    })
                })
                .take(3) // Match the primary provider's per-part-of-speech limit.
                .collect();

            if definitions.is_empty() {
                return None;
            }

            Some(DictionaryMeaning {
                part_of_speech: entry
                    .part_of_speech
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                definitions,
            })
        })
        .collect();

    if meanings.is_empty() {
        return Err(ShioriError::Other(format!(
            "No definition found for \"{}\"",
            word
        )));
    }

    Ok(DictionaryResult {
        word: word.to_string(),
        phonetic: None,
        audio_url: None,
        meanings,
        source_url: Some(format!("https://en.wiktionary.org/wiki/{}", word)),
    })
}

/// Translate text using MyMemory API (primary) with Lingva fallback.
/// source_lang: ISO 639-1 code (e.g. "en") or "auto" for auto-detect
/// target_lang: ISO 639-1 code (e.g. "es")
pub async fn translate_text(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    if text.trim().is_empty() {
        return Ok(TranslationResult {
            translated_text: text.to_string(),
            source_language: source_lang.to_string(),
            target_language: target_lang.to_string(),
            provider: "none".to_string(),
        });
    }

    let key = format!("{}:{}:{}", source_lang, target_lang, text);
    if let Some(cached) = get_translation_cache().lock().unwrap().get(&key) {
        return Ok(cached.clone());
    }

    let result = if text.len() > 400 {
        translate_long_text(text, source_lang, target_lang).await?
    } else {
        translate_text_single(text, source_lang, target_lang).await?
    };

    get_translation_cache().lock().unwrap().insert(key, result.clone());
    Ok(result)
}

async fn translate_long_text(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    let mut translated_pieces = Vec::new();
    let mut provider = String::new();

    // Naive split by ". " to chunk long paragraphs
    let chunks: Vec<&str> = text.split(". ").collect();
    
    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.trim().is_empty() {
            continue;
        }
        // Avoid nested loops calling themselves infinitely; chunks should be < 400 chars now.
        // If still > 400, truncate to 400 to prevent failure.
        let safe_chunk = if chunk.len() > 400 { &chunk[..400] } else { chunk };
        
        let res = translate_text_single(safe_chunk, source_lang, target_lang).await?;
        translated_pieces.push(res.translated_text);
        if provider.is_empty() {
            provider = res.provider;
        }
        
        // Respect API rate limits somewhat by delaying slightly between chunks
        if i < chunks.len() - 1 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    Ok(TranslationResult {
        translated_text: translated_pieces.join(". "),
        source_language: source_lang.to_string(),
        target_language: target_lang.to_string(),
        provider,
    })
}

async fn translate_text_single(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    // Skip MyMemory for "auto" source — it requires specific ISO language codes
    if source_lang != "auto" {
        match translate_mymemory(text, source_lang, target_lang).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!("MyMemory translation failed: {}, trying Google fallback", e);
            }
        }
    }

    // Fallback 1: Google Translate API (free)
    match translate_google_free(text, source_lang, target_lang).await {
        Ok(result) => return Ok(result),
        Err(e) => {
            log::warn!("Google translation failed: {}, trying Lingva fallback", e);
        }
    }

    // Fallback 2: Lingva
    match translate_lingva(text, source_lang, target_lang).await {
        Ok(result) => Ok(result),
        Err(e) => Err(ShioriError::Other(format!(
            "All translation providers failed. Last error: {}",
            e
        ))),
    }
}

/// Translate using MyMemory API
async fn translate_mymemory(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    let client =
        build_client().map_err(|e| ShioriError::Other(format!("HTTP client error: {}", e)))?;

    let langpair = format!("{}|{}", source_lang, target_lang);

    let response = client
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text), ("langpair", &langpair)])
        .send()
        .await
        .map_err(|e| ShioriError::Other(format!("MyMemory request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(ShioriError::Other(format!(
            "MyMemory API returned status {}",
            response.status()
        )));
    }

    let result: MyMemoryResponse = response
        .json()
        .await
        .map_err(|e| ShioriError::Other(format!("Failed to parse MyMemory response: {}", e)))?;

    // Check for error status in response body
    if let Some(status) = &result.response_status {
        if let Some(status_num) = status.as_u64() {
            if status_num == 403 {
                return Err(ShioriError::Other("MyMemory quota exceeded".to_string()));
            }
        }
    }

    Ok(TranslationResult {
        translated_text: result.response_data.translated_text,
        source_language: source_lang.to_string(),
        target_language: target_lang.to_string(),
        provider: "mymemory".to_string(),
    })
}

/// Translate using Google Translate API (free)
async fn translate_google_free(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    let client =
        build_client().map_err(|e| ShioriError::Other(format!("HTTP client error: {}", e)))?;
    let url = "https://translate.googleapis.com/translate_a/single";

    let response = client
        .get(url)
        .query(&[
            ("client", "gtx"),
            ("sl", source_lang),
            ("tl", target_lang),
            ("dt", "t"),
            ("q", text),
        ])
        .send()
        .await
        .map_err(|e| ShioriError::Other(format!("Google Translate request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(ShioriError::Other(format!(
            "Google Translate API returned status {}",
            response.status()
        )));
    }

    let json_val: serde_json::Value = response.json().await.map_err(|e| {
        ShioriError::Other(format!("Failed to parse Google Translate response: {}", e))
    })?;

    let mut translated_text = String::new();

    // The response is an array: [[["translated_sentence_1", ...], ["translated_sentence_2", ...]], ...]
    if let Some(sentences) = json_val
        .as_array()
        .and_then(|a| a.get(0))
        .and_then(|v| v.as_array())
    {
        for sentence_group in sentences {
            if let Some(sentence) = sentence_group
                .as_array()
                .and_then(|a| a.get(0))
                .and_then(|s| s.as_str())
            {
                translated_text.push_str(sentence);
            }
        }
    }

    if translated_text.is_empty() {
        return Err(ShioriError::Other(
            "Empty translation from Google Translate".to_string(),
        ));
    }

    Ok(TranslationResult {
        translated_text,
        source_language: source_lang.to_string(),
        target_language: target_lang.to_string(),
        provider: "google".to_string(),
    })
}

/// Translate using Lingva API (fallback)
async fn translate_lingva(
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslationResult> {
    let client =
        build_client().map_err(|e| ShioriError::Other(format!("HTTP client error: {}", e)))?;

    let instances = [
        "https://lingva.thedesk.top",
        "https://lingva.lunar.icu",
        "https://translate.plausibility.cloud",
        "https://lingva.garudalinux.org",
    ];

    let mut last_error = String::new();

    for instance in instances {
        let url = format!(
            "{}/api/v1/{}/{}/{}",
            instance,
            urlencoding::encode(source_lang),
            urlencoding::encode(target_lang),
            urlencoding::encode(text)
        );

        let response = match client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = format!("Request failed: {}", e);
                log::warn!("Lingva instance {} failed: {}", instance, e);
                continue;
            }
        };

        if !response.status().is_success() {
            last_error = format!("Status {}", response.status());
            log::warn!(
                "Lingva instance {} failed with status {}",
                instance,
                response.status()
            );
            continue;
        }

        let result: LingvaResponse = match response.json().await {
            Ok(res) => res,
            Err(e) => {
                last_error = format!("Parse failed: {}", e);
                log::warn!("Lingva instance {} parse failed: {}", instance, e);
                continue;
            }
        };

        return Ok(TranslationResult {
            translated_text: result.translation,
            source_language: source_lang.to_string(),
            target_language: target_lang.to_string(),
            provider: "lingva".to_string(),
        });
    }

    Err(ShioriError::Other(format!(
        "All Lingva instances failed. Last error: {}",
        last_error
    )))
}
