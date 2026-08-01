/// Roundtrip tests for the EPUB builder and OEB structs.
///
/// Each supported source format is parsed from the shared fixtures in
/// `broken-files/samples/`, assembled into an EPUB and read back with the
/// `epub` crate to assert chapter structure and content survived.

#[cfg(test)]
pub mod tests {
    use crate::conversion::{
        epub_builder, formats,
        oeb::{OebBook, OebChapter},
    };
    use std::path::{Path, PathBuf};

    /// Absolute path to a fixture in `broken-files/samples/`.
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../broken-files/samples")
            .join(name)
    }

    /// Absolute path to a fixture in `broken-files/`.
    fn broken_fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../broken-files")
            .join(name)
    }

    /// Build an EPUB from a parsed book and return (epub path, full text of all chapters).
    fn build_and_read(book: &mut OebBook) -> (PathBuf, String) {
        book.sanitize_html();
        let tmp = std::env::temp_dir().join(format!(
            "shiori_test_roundtrip_{}.epub",
            uuid::Uuid::new_v4()
        ));
        epub_builder::build_epub(book, &tmp).expect("build_epub failed");
        assert!(tmp.exists(), "EPUB file was not created");

        let mut doc = ::epub::doc::EpubDoc::new(&tmp).expect("epub crate failed to open output");
        let mut full_text = String::new();
        for i in 0..doc.get_num_chapters() {
            doc.set_current_chapter(i);
            if let Some((content, _mime)) = doc.get_current_str() {
                full_text.push_str(&content);
                full_text.push('\n');
            }
        }
        (tmp, full_text)
    }

    fn assert_chapter_text(book: &mut OebBook, expected: &[&str]) {
        let (tmp, full_text) = build_and_read(book);
        for fragment in expected {
            assert!(
                full_text.contains(fragment),
                "EPUB text missing {:?}. Got: {}",
                fragment,
                full_text.chars().take(500).collect::<String>()
            );
        }
        std::fs::remove_file(tmp).unwrap();
    }

    #[test]
    fn test_epub_builder_roundtrip() {
        let mut book = OebBook::new("Test Book");
        book.authors = vec!["Test Author".to_string()];
        book.language = "en".to_string();
        book.chapters.push(OebChapter {
            id: "chapter_001".to_string(),
            title: Some("Chapter 1".to_string()),
            html: "<p>Hello, world.</p>".to_string(),
        });

        let tmp = std::env::temp_dir().join("shiori_test_roundtrip.epub");
        epub_builder::build_epub(&book, &tmp).expect("build_epub failed");
        assert!(tmp.exists(), "EPUB file was not created");

        let file = std::fs::File::open(&tmp).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        assert!(archive.by_name("mimetype").is_ok(), "mimetype missing");
        assert!(
            archive.by_name("META-INF/container.xml").is_ok(),
            "container.xml missing"
        );
        assert!(
            archive.by_name("OEBPS/content.opf").is_ok(),
            "content.opf missing"
        );
        assert!(
            archive.by_name("OEBPS/nav.xhtml").is_ok(),
            "nav.xhtml missing"
        );
        assert!(archive.by_name("OEBPS/toc.ncx").is_ok(), "toc.ncx missing");
        assert!(
            archive.by_name("OEBPS/Text/chapter_001.xhtml").is_ok(),
            "chapter_001.xhtml missing"
        );

        // Verify mimetype is uncompressed (EPUB spec requirement)
        let mimetype_entry = archive.by_name("mimetype").unwrap();
        assert_eq!(
            mimetype_entry.compression(),
            zip::CompressionMethod::Stored,
            "mimetype must be stored (uncompressed)"
        );

        std::fs::remove_file(tmp).unwrap();
    }

    #[test]
    fn test_oeb_sanitize_removes_script() {
        let mut book = OebBook::new("Test");
        book.add_chapter(
            Some("Ch1".to_string()),
            r#"<p>Hello</p><script>alert(1)</script><p>World</p>"#.to_string(),
        );
        book.sanitize_html();
        let html = &book.chapters[0].html;
        assert!(!html.contains("<script>"), "script tag should be removed");
        assert!(html.contains("Hello") && html.contains("World"));
    }

    #[tokio::test]
    async fn test_unsupported_format_error() {
        let result =
            crate::conversion::convert_to_epub_new(std::path::Path::new("test.xyz"), None, None)
                .await;
        assert!(
            matches!(
                result,
                Err(crate::conversion::ConversionError::UnsupportedFormat(_))
            ),
            "Expected UnsupportedFormat error"
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    // PER-FORMAT ROUND-TRIPS (fixtures from broken-files/samples/)
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn test_pdf_roundtrip() {
        let mut book = formats::pdf::parse(&fixture("book.pdf")).expect("pdf parse failed");
        assert!(!book.chapters.is_empty(), "pdf produced no chapters");
        assert_chapter_text(&mut book, &["Chapter One", "Once upon a time"]);
    }

    #[test]
    fn test_docx_roundtrip() {
        let mut book = formats::docx::parse(&fixture("book.docx")).expect("docx parse failed");
        assert!(!book.chapters.is_empty(), "docx produced no chapters");
        assert_chapter_text(&mut book, &["Chapter One", "docx content"]);
    }

    #[test]
    fn test_fb2_roundtrip() {
        let mut book = formats::fb2::parse(&fixture("book.fb2")).expect("fb2 parse failed");
        assert!(!book.chapters.is_empty(), "fb2 produced no chapters");
        assert_eq!(book.title, "Sample Book", "fb2 title from metadata");
        assert_chapter_text(&mut book, &["Chapter One", "test book in FB2"]);
    }

    #[test]
    fn test_txt_roundtrip() {
        let mut book = formats::txt::parse(&fixture("book.txt")).expect("txt parse failed");
        assert!(!book.chapters.is_empty(), "txt produced no chapters");
        assert_chapter_text(&mut book, &["Chapter One", "Once upon a time"]);
    }

    #[test]
    fn test_html_roundtrip() {
        let mut book = formats::html::parse(&fixture("book.html")).expect("html parse failed");
        assert!(!book.chapters.is_empty(), "html produced no chapters");
        // First h1 is the title; h2s become chapters.
        assert_eq!(book.title, "Sample Book", "html title from <title> tag");
        assert_chapter_text(&mut book, &["Chapter One", "enough text to paginate"]);
    }

    #[test]
    fn test_markdown_roundtrip() {
        let mut book = formats::markdown::parse(&fixture("book.md")).expect("md parse failed");
        assert!(!book.chapters.is_empty(), "md produced no chapters");
        assert_eq!(book.title, "Sample Book", "md title from first heading");
        // # Sample Book is the title; ## Chapter One/Two/Three are chapters.
        assert_eq!(book.chapters.len(), 3, "three h2 chapters expected");
        assert_chapter_text(
            &mut book,
            &["Chapter One", "Markdown chapter one content here."],
        );
    }

    #[test]
    fn test_convert_to_epub_new_markdown() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../broken-files/samples/book.md");
        let out = crate::conversion::convert_to_epub_new(&path, None, None);
        let out = futures::executor::block_on(out).expect("md conversion failed");
        assert!(out.exists());

        let mut doc = ::epub::doc::EpubDoc::new(&out).expect("epub crate failed to open output");
        let mut full_text = String::new();
        for i in 0..doc.get_num_chapters() {
            doc.set_current_chapter(i);
            if let Some((content, _)) = doc.get_current_str() {
                full_text.push_str(&content);
            }
        }
        assert!(full_text.contains("Chapter One"));
        assert!(full_text.contains("Markdown chapter one content here."));

        // Clean up the temp conversion directory.
        if let Some(parent) = out.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // MOBI — stub fixture errors gracefully, real fixture round-trips
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mobi_corrupt_input_errors() {
        // book.mobi is a 14-byte "404: Not Found" stub — must fail cleanly.
        let result = formats::mobi::parse(&fixture("book.mobi"));
        assert!(
            result.is_err(),
            "corrupt/missing mobi must produce a conversion error"
        );
    }

    #[test]
    fn test_mobi_real_roundtrip() {
        let real = broken_fixture("1752426479_the_briar_club_-_kate_quinn.mobi");
        if !real.exists() {
            eprintln!("SKIP: real mobi fixture not present");
            return;
        }
        let mut book = formats::mobi::parse(&real).expect("real mobi parse failed");
        assert!(!book.chapters.is_empty(), "real mobi produced no chapters");
        assert_chapter_text(&mut book, &["Briar"]);
    }
}
