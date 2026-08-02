//! Regression tests for the minimal-but-valid DOCX bug:
//! broken-files/samples/book.docx (only [Content_Types].xml, _rels/.rels and
//! word/document.xml) fails with "Invalid DOCX file: Failed to read from zip."
//! because docx-rs 0.4 cannot parse the minimal structure.
//!
//! Fix: DocxAdapter::load falls back to a direct ZIP + quick-xml parse, and
//! DocxFormatAdapter::validate accepts any structurally valid ZIP containing
//! word/document.xml. A truly corrupt file must still error cleanly.

use shiori::services::adapters::docx::DocxFormatAdapter;
use shiori::services::docx_adapter::DocxAdapter;
use shiori::services::format_adapter::BookFormatAdapter;
use shiori::services::renderer::BookReaderAdapter;
use std::path::Path;

const SAMPLE_DOCX: &str = "/home/zura/Personal/coding_cuff/Shiori/broken-files/samples/book.docx";

#[tokio::test]
async fn docx_reader_loads_minimal_docx() {
    if !Path::new(SAMPLE_DOCX).exists() {
        eprintln!("skipping: file missing");
        return;
    }
    let mut adapter = DocxAdapter::new();
    adapter
        .load(SAMPLE_DOCX)
        .await
        .expect("minimal docx must load");

    let chapter = adapter.get_chapter(0).expect("chapter 0");
    assert!(
        chapter.content.contains("Sample Book") || chapter.content.contains("Chapter One"),
        "chapter content should contain book text, got: {}",
        &chapter.content[..chapter.content.len().min(300)]
    );
    assert_eq!(adapter.chapter_count(), 1);
}

#[tokio::test]
async fn docx_format_adapter_accepts_minimal_docx() {
    if !Path::new(SAMPLE_DOCX).exists() {
        eprintln!("skipping: file missing");
        return;
    }
    let adapter = DocxFormatAdapter::new();
    let result = adapter
        .validate(Path::new(SAMPLE_DOCX))
        .await
        .expect("minimal docx must validate");
    assert!(
        result.word_count.unwrap_or(0) > 0,
        "word_count should be > 0 for the sample"
    );
}

#[tokio::test]
async fn docx_reader_rejects_corrupt_file() {
    let dir = std::env::temp_dir().join("shiori_fix_docx_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let corrupt = dir.join("corrupt.docx");
    std::fs::write(&corrupt, b"this is not a zip file at all").expect("write garbage");

    let mut adapter = DocxAdapter::new();
    let err = adapter.load(corrupt.to_str().unwrap()).await;
    assert!(err.is_err(), "garbage bytes must produce Err, not panic");
}

#[tokio::test]
async fn docx_format_adapter_rejects_corrupt_file() {
    let dir = std::env::temp_dir().join("shiori_fix_docx_test");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let corrupt = dir.join("corrupt.docx");
    std::fs::write(&corrupt, b"this is not a zip file at all").expect("write garbage");

    let adapter = DocxFormatAdapter::new();
    let err = adapter.validate(&corrupt).await;
    assert!(err.is_err(), "garbage bytes must produce Err, not panic");
}
