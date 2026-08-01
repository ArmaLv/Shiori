//! Cover slice tests: embedded extraction (docx / fb2 / markdown), online
//! lookup error path (never hits real APIs), CoverService byte storage, and
//! the import-path cover wiring.
use base64::Engine as _;
use shiori::db::Database;
use shiori::services::cover_service::CoverService;
use shiori::services::{library_service, metadata_service};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("shiori_cover_test_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 1x1 transparent PNG (validated by `file` — PNG, 1x1, RGBA).
fn tiny_png() -> Vec<u8> {
    const B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
    base64::engine::general_purpose::STANDARD
        .decode(B64)
        .expect("valid base64 png")
}

/// Minimal-but-valid DOCX: content types, package rels, one paragraph plus an
/// inline drawing referencing media/image1.png through document rels.
fn build_docx_with_image(png: &[u8]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )
    .unwrap();

    zip.start_file("_rels/.rels", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:t>Cover image below</w:t></w:r></w:p>
    <w:p><w:r><w:drawing><a:inline><a:graphic><a:graphicData><pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:blipFill><a:blip r:embed="rId5"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></a:inline></w:drawing></w:r></w:p>
  </w:body>
</w:document>"#,
    )
    .unwrap();

    zip.start_file("word/_rels/document.xml.rels", options)
        .unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
</Relationships>"#,
    )
    .unwrap();

    zip.start_file("word/media/image1.png", options).unwrap();
    zip.write_all(png).unwrap();

    zip.finish().unwrap().into_inner()
}

fn assert_webp_cover(result: Option<String>, what: &str) -> PathBuf {
    let cover_path = result.unwrap_or_else(|| panic!("{} should yield a cover", what));
    let bytes = fs::read(&cover_path).unwrap_or_else(|e| panic!("cover readable: {}", e));
    assert!(bytes.len() > 20, "cover should have content");
    assert_eq!(&bytes[0..4], b"RIFF", "{} cover should be webp", what);
    assert_eq!(&bytes[8..12], b"WEBP");
    PathBuf::from(cover_path)
}

#[test]
fn docx_with_embedded_image_extracts_webp_cover() {
    let dir = temp_dir("docx");
    let docx_path = dir.join("book.docx");
    fs::write(&docx_path, build_docx_with_image(&tiny_png())).unwrap();

    let result = metadata_service::extract_cover(
        &docx_path.to_string_lossy(),
        "11111111-1111-1111-1111-111111111111",
        &dir,
    )
    .expect("extract_cover should not error");
    assert_webp_cover(result, "docx");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn fb2_with_base64_cover_extracts_webp_cover() {
    let dir = temp_dir("fb2");
    let fb2_path = dir.join("book.fb2");
    let png_b64 = base64::engine::general_purpose::STANDARD.encode(tiny_png());
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
<description><title-info><book-title>Cover Book</book-title><coverpage><image l:href="#cover.png"/></coverpage></title-info></description>
<body><section><title><p>Chapter</p></title><p>Text</p></section></body>
<binary id="cover.png" content-type="image/png">{}</binary>
</FictionBook>"##,
        png_b64
    );
    fs::write(&fb2_path, fb2).unwrap();

    let result = metadata_service::extract_cover(
        &fb2_path.to_string_lossy(),
        "22222222-2222-2222-2222-222222222222",
        &dir,
    )
    .expect("extract_cover should not error");
    assert_webp_cover(result, "fb2");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn fb2_without_coverpage_falls_back_to_first_binary_image() {
    let dir = temp_dir("fb2-fallback");
    let fb2_path = dir.join("book.fb2");
    let png_b64 = base64::engine::general_purpose::STANDARD.encode(tiny_png());
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
<description><title-info><book-title>No Coverpage</book-title></title-info></description>
<body><section><title><p>Chapter</p></title><p><image l:href="#pic1.png"/></p></section></body>
<binary id="pic1.png" content-type="image/png">{}</binary>
</FictionBook>"##,
        png_b64
    );
    fs::write(&fb2_path, fb2).unwrap();

    let result = metadata_service::extract_cover(
        &fb2_path.to_string_lossy(),
        "33333333-3333-3333-3333-333333333333",
        &dir,
    )
    .expect("extract_cover should not error");
    assert_webp_cover(result, "fb2 body image");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn markdown_never_has_embedded_cover() {
    let dir = temp_dir("md");
    let md_path = dir.join("book.md");
    fs::write(&md_path, "# Title\n\nSome text\n").unwrap();

    let result = metadata_service::extract_cover(
        &md_path.to_string_lossy(),
        "44444444-4444-4444-4444-444444444444",
        &dir,
    )
    .expect("extract_cover should not error");
    assert!(result.is_none(), "markdown has no embedded cover");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn import_single_book_extracts_docx_cover_and_sets_cover_path() {
    let dir = temp_dir("import-docx");
    let db = Database::new(&dir.join("test.db")).unwrap();
    let docx_path = dir.join("book.docx");
    fs::write(&docx_path, build_docx_with_image(&tiny_png())).unwrap();

    let is_duplicate =
        library_service::import_single_book(&db, &docx_path.to_string_lossy(), &dir).unwrap();
    assert!(!is_duplicate);

    let books = library_service::get_all_books(&db, 10, 0).unwrap();
    assert_eq!(books.len(), 1);
    let cover_path = books[0]
        .cover_path
        .clone()
        .expect("docx import should set a cover path");
    assert!(std::path::Path::new(&cover_path).exists());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn update_book_cover_path_persists() {
    let dir = temp_dir("update-path");
    let db = Database::new(&dir.join("test.db")).unwrap();
    let md_path = dir.join("book.md");
    fs::write(&md_path, "# Title\n").unwrap();
    library_service::import_single_book(&db, &md_path.to_string_lossy(), &dir).unwrap();

    let books = library_service::get_all_books(&db, 10, 0).unwrap();
    let id = books[0].id.unwrap();
    assert!(
        books[0].cover_path.is_none(),
        "md import has no embedded cover"
    );

    library_service::update_book_cover_path(&db, id, Some("/tmp/fake-cover.webp")).unwrap();
    let books = library_service::get_all_books(&db, 10, 0).unwrap();
    assert_eq!(books[0].cover_path.as_deref(), Some("/tmp/fake-cover.webp"));
    fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn online_cover_lookup_fails_gracefully_without_network() {
    use shiori::services::online_cover::OnlineCoverClient;

    // Point every endpoint at a dead local port — connection refused is
    // instant, so this never touches the real APIs and never hangs.
    let client = OnlineCoverClient::new_with_urls(
        "http://127.0.0.1:1/googlebooks".to_string(),
        "http://127.0.0.1:1/search".to_string(),
        "http://127.0.0.1:1/covers".to_string(),
        std::time::Duration::from_millis(500),
    )
    .unwrap();

    let result = client
        .fetch_cover("Some Book Title", Some("Some Author"), None)
        .await;
    assert!(result.is_ok(), "network failure must not error out");
    assert!(result.unwrap().is_none(), "no cover available offline");
}

#[tokio::test]
async fn store_cover_bytes_writes_full_cover_set() {
    let dir = temp_dir("store");
    let service = CoverService::new(dir.join("storage")).unwrap();
    let uuid = Uuid::new_v4();

    let set = service.store_cover_bytes(uuid, tiny_png()).await.unwrap();
    assert!(set.thumbnail.exists(), "thumb.webp should exist");
    assert!(set.medium.exists(), "medium.webp should exist");
    assert!(set.full.exists(), "full.webp should exist");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn online_cover_attempt_guard_marks_book_once() {
    use shiori::services::online_cover::try_begin_online_cover_attempt;

    // Unique id so it can't collide with other tests sharing the global set.
    let id = i64::MAX - 424242;
    assert!(try_begin_online_cover_attempt(id), "first attempt allowed");
    assert!(
        !try_begin_online_cover_attempt(id),
        "second attempt blocked per run"
    );
}
