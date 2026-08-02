//! Regression tests for two verified bugs:
//! 1. Minimal-but-valid DOCX files fail to open ("Invalid DOCX file: Failed
//!    to read from zip.") because docx-rs 0.4 cannot parse them.
//! 2. Real MOBI files render garbled text (custom PDB extractor wins the
//!    scoring over the `mobi` crate output), and bogus MOBI files (failed
//!    download artifacts, e.g. ASCII "404: Not Found") must error cleanly.

use shiori::services::adapters::docx::DocxFormatAdapter;
use shiori::services::docx_adapter::DocxAdapter;
use shiori::services::format_adapter::BookFormatAdapter;
use shiori::services::mobi_adapter::MobiAdapter;
use shiori::services::renderer::BookReaderAdapter;
use std::path::Path;

const SAMPLE_DOCX: &str = "/home/zura/Personal/coding_cuff/Shiori/broken-files/samples/book.docx";
const REAL_MOBI: &str =
    "/home/zura/Personal/coding_cuff/Shiori/broken-files/1752426479_the_briar_club_-_kate_quinn.mobi";
const BOGUS_MOBI: &str = "/home/zura/Personal/coding_cuff/Shiori/broken-files/samples/book.mobi";

#[tokio::test]
async fn docx_reader_loads_minimal_docx() {
    if !Path::new(SAMPLE_DOCX).exists() {
        eprintln!("skipping: file missing");
        return;
    }
    let mut adapter = DocxAdapter::new();
    adapter.load(SAMPLE_DOCX).await.expect("minimal docx must load");

    let chapter = adapter.get_chapter(0).expect("chapter 0");
    let text = chapter.content;
    assert!(
        text.contains("Sample Book") || text.contains("Chapter One"),
        "chapter content should contain book text, got: {}",
        &text[..text.len().min(300)]
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
    assert!(result.word_count.unwrap_or(0) > 0);
}

#[tokio::test]
async fn mobi_reader_loads_real_mobi_as_prose() {
    if !Path::new(REAL_MOBI).exists() {
        eprintln!("skipping: file missing");
        return;
    }
    let mut adapter = MobiAdapter::new();
    adapter.load(REAL_MOBI).await.expect("real mobi must load");

    // Regression: the garbled decoder produced ~5 mangled chapters with
    // corrupt markup; the fixed PalmDOC/hybrid decode yields the full novel.
    assert!(
        adapter.chapter_count() >= 20,
        "expected a full book of chapters, got {}",
        adapter.chapter_count()
    );
    let chapter = adapter.get_chapter(0).expect("chapter 0");
    let content = chapter.content;
    let plain = content
        .replace("<p>", " ")
        .replace("</p>", " ")
        .replace("<br", " ")
        .replace("<div>", " ")
        .replace("</div>", " ");
    assert!(
        plain.len() > 200,
        "chapter 0 should be non-trivial prose, got {} chars: {}",
        plain.len(),
        &plain[..plain.len().min(300)]
    );
    // The old garbage output contained these corrupt markers.
    for marker in ["6gn=", "<r<", "enct", "filep0767", "404: Not Found"] {
        assert!(
            !plain.contains(marker),
            "chapter 0 must not contain corrupt marker {marker:?}"
        );
    }
    // The book's actual prose must be present (dedication text).
    let first_three: String = (0..adapter.chapter_count().min(3))
        .filter_map(|i| adapter.get_chapter(i).ok())
        .map(|c| c.content)
        .collect();
    assert!(
        first_three.contains("Briar Club"),
        "book prose should mention 'Briar Club'"
    );
    let replacement_run = "\u{FFFD}".repeat(8);
    assert!(
        !plain.contains(&replacement_run),
        "must not contain long runs of replacement chars"
    );
}

#[tokio::test]
async fn mobi_reader_rejects_bogus_mobi() {
    if !Path::new(BOGUS_MOBI).exists() {
        eprintln!("skipping: file missing");
        return;
    }
    let mut adapter = MobiAdapter::new();
    let err = adapter.load(BOGUS_MOBI).await;
    assert!(
        err.is_err(),
        "bogus mobi (ASCII '404: Not Found') must return Err"
    );
    let err = err.unwrap_err();
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("MOBI") || msg.contains("mobi"),
        "error should mention MOBI, got: {}",
        msg
    );
}
