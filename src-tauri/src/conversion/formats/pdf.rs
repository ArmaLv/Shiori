/// PDF → OEB parser.
///
/// Uses the legacy `crate::conversion::pdf::parse` pipeline (pdftohtml when
/// available, lopdf otherwise) which produces chapter-split XHTML with images.
/// When that fails — most commonly because the `pdftohtml` binary is not
/// installed — falls back to a pure-Rust path through `pdf-extract`
/// (the same extractor the native reader adapter uses).
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::{OebBook, OebChapter};
use crate::conversion::utils;
use crate::services::adapters::PdfFormatAdapter;
use crate::services::format_adapter::BookFormatAdapter;

/// Parse a PDF file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    let adapter = PdfFormatAdapter::new();
    common::block_on(async move {
        match crate::conversion::pdf::parse(&path_buf, None).await {
            Ok(book) => Ok(book),
            Err(legacy_err) => {
                log::warn!(
                    "[PDF→EPUB] Legacy parse failed ({}), falling back to pdf-extract",
                    legacy_err
                );
                parse_with_pdf_extract(&path_buf, &adapter).await
            }
        }
    })?
}

/// Pure-Rust fallback: pdf-extract text → post-process → chapter split.
async fn parse_with_pdf_extract(
    path: &Path,
    adapter: &PdfFormatAdapter,
) -> Result<OebBook, ConversionError> {
    let text =
        PdfFormatAdapter::extract_content(path).map_err(|e| ConversionError::ParseError {
            format: "PDF".to_string(),
            detail: e.to_string(),
        })?;

    if text.trim().is_empty() {
        return Err(ConversionError::EmptyContent);
    }

    let meta = adapter.extract_metadata(path).await.unwrap_or_default();

    let title = if !meta.title.is_empty() && meta.title != "Unknown" {
        meta.title
    } else {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.replace('_', " ").replace('-', " "))
            .unwrap_or_else(|| "Untitled".to_string())
    };

    let processed = PdfFormatAdapter::post_process_text(&text);
    let html = utils::text_to_html_paragraphs(&processed);
    let chapters = split_chapters(&html);

    let mut book = OebBook::new(title);
    book.authors = meta.authors.clone();
    book.language = meta.language.clone().unwrap_or_else(|| "en".to_string());
    book.description = meta.description.clone();

    for (i, (ch_title, ch_body)) in chapters.into_iter().enumerate() {
        book.chapters.push(OebChapter {
            id: format!("chapter_{:03}", i + 1),
            title: Some(ch_title),
            html: ch_body,
        });
    }

    Ok(book)
}

/// Split `utils::text_to_html_paragraphs`-style HTML into chapters using the
/// shared heading heuristic (`utils::looks_like_heading`).
fn split_chapters(html: &str) -> Vec<(String, String)> {
    static PARAGRAPH_RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap());

    let mut chapters: Vec<(String, String)> = Vec::new();
    let mut current_title = "Document".to_string();
    let mut current_body = String::new();

    for line in html.lines() {
        let candidate = PARAGRAPH_RE
            .captures(line)
            .map(|cap| utils::strip_html_tags(&cap[1]).trim().to_string())
            .unwrap_or_default();

        let is_heading = !candidate.is_empty()
            && utils::looks_like_heading(&candidate)
            && candidate.len() <= 120
            && !candidate.contains('@')
            && !candidate.contains("://");

        if is_heading {
            if !current_body.trim().is_empty() {
                chapters.push((current_title.clone(), current_body.trim().to_string()));
                current_body.clear();
            }
            current_title = candidate;
            current_body.push_str(&format!(
                "  <h2>{}</h2>\n",
                crate::conversion::oeb::escape_xml(&current_title)
            ));
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }

    if !current_body.trim().is_empty() {
        chapters.push((current_title, current_body.trim().to_string()));
    }

    if chapters.is_empty() {
        chapters.push(("Document".to_string(), html.to_string()));
    }

    chapters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_chapters_detects_headings() {
        let html = "<p>Introduction</p>\n<p>Some intro text here.</p>\n<p>Chapter 1 The Beginning</p>\n<p>Once upon a time.</p>";
        let chapters = split_chapters(html);
        assert!(chapters.len() >= 2);
        let titles: Vec<&str> = chapters.iter().map(|(t, _)| t.as_str()).collect();
        assert!(titles.iter().any(|t| t.contains("Chapter 1")));
        assert!(chapters[0].1.contains("Some intro text"));
    }
}
