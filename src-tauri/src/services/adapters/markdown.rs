#![allow(dead_code)]
/// Markdown Format Adapter - Markdown text file support
///
/// Provides metadata inference, validation, and (engine-mediated) conversion
/// to EPUB for `.md` / `.markdown` files.
use crate::services::format_adapter::{
    BookFormatAdapter, BookMetadata, ConversionResult, CoverImage, FormatCapabilities, FormatError,
    FormatResult, ValidationResult,
};
use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;

pub struct MarkdownFormatAdapter;

impl MarkdownFormatAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BookFormatAdapter for MarkdownFormatAdapter {
    fn format_id(&self) -> &str {
        "markdown"
    }

    async fn validate(&self, path: &Path) -> FormatResult<ValidationResult> {
        let mut file = fs::File::open(path).await?;
        let file_size = file.metadata().await?.len();

        // Read first 4KB to validate it's text
        let mut buffer = vec![0u8; 4096];
        let bytes_read = file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);

        // Check if valid UTF-8
        match std::str::from_utf8(&buffer) {
            Ok(_) => {
                let mut result = ValidationResult::valid(file_size);

                let file_data = fs::read(path).await?;
                let content = String::from_utf8_lossy(&file_data).into_owned();
                result.word_count = Some(count_words(&content));
                result.page_count = Some(estimate_pages(&content));

                Ok(result)
            }
            Err(e) => Ok(ValidationResult::invalid(format!(
                "Invalid UTF-8 encoding: {}",
                e
            ))),
        }
    }

    async fn extract_metadata(&self, path: &Path) -> FormatResult<BookMetadata> {
        let file_size = fs::metadata(path).await?.len();
        let file_data = fs::read(path).await?;
        let content = String::from_utf8_lossy(&file_data).into_owned();

        // Infer title from the first top-level heading, else the filename
        let title = extract_first_heading(&content)
            .or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Untitled".to_string());

        let word_count = Some(count_words(&content));
        let page_count = Some(estimate_pages(&content));

        Ok(BookMetadata {
            title,
            authors: vec![],
            publisher: None,
            pubdate: None,
            isbn: None,
            language: Some("en".to_string()),
            description: None,
            tags: vec![],
            series: None,
            series_index: None,
            rating: None,
            file_format: "markdown".to_string(),
            file_size,
            page_count,
            word_count,
        })
    }

    async fn extract_cover(&self, _path: &Path) -> FormatResult<Option<CoverImage>> {
        // Markdown has no embedded cover — fallback path only (another slice's job)
        Ok(None)
    }

    fn can_convert_to(&self, target: &str) -> bool {
        matches!(target, "epub")
    }

    async fn convert_to(
        &self,
        _source: &Path,
        _target: &Path,
        target_format: &str,
    ) -> FormatResult<ConversionResult> {
        if !self.can_convert_to(target_format) {
            return Err(FormatError::ConversionNotSupported {
                from: "markdown".to_string(),
                to: target_format.to_string(),
            });
        }

        // Conversion will be handled by ConversionEngine
        Err(FormatError::ConversionError(
            "Conversion not yet implemented. Use ConversionEngine.".to_string(),
        ))
    }

    fn capabilities(&self) -> FormatCapabilities {
        FormatCapabilities {
            supports_toc: true,     // headings become a TOC in the reader
            supports_images: false, // handled at render time by the reader
            supports_text_reflow: true,
            supports_annotations: true,
            supports_metadata: false, // limited metadata
            is_readable: true,
            supports_search: true,
        }
    }
}

/// Count words in text
fn count_words(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

/// Estimate page count (assuming ~250 words per page)
fn estimate_pages(text: &str) -> u32 {
    let words = count_words(text);
    (words / 250).max(1)
}

/// Extract the first `# ` (or `## `, `### `) heading as a title candidate
fn extract_first_heading(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        let heading = trimmed
            .strip_prefix("# ")
            .or_else(|| trimmed.strip_prefix("## "))
            .or_else(|| trimmed.strip_prefix("### "));
        if let Some(h) = heading {
            let h = h.trim();
            if !h.is_empty() {
                return Some(h.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("Hello world"), 2);
        assert_eq!(count_words("  Multiple   spaces  "), 2);
    }

    #[test]
    fn test_extract_first_heading() {
        assert_eq!(
            extract_first_heading("# My Book\n\nSome text"),
            Some("My Book".to_string())
        );
        assert_eq!(
            extract_first_heading("## Chapter One\n\nText"),
            Some("Chapter One".to_string())
        );
        assert_eq!(extract_first_heading("No heading here\nJust text"), None);
    }
}
