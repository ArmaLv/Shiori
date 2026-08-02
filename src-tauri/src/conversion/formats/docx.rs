/// DOCX → OEB parser.
///
/// Delegates to the legacy `crate::conversion::docx::parse` pipeline, which
/// walks `word/document.xml` paragraph-by-paragraph with style inheritance,
/// list state, image extraction via relationships, footnotes and page-break
/// chapter boundaries.
///
/// A metadata/title post-pass then fixes the legacy title inference:
/// - `docProps/core.xml` `dc:title` / `dc:creator` win when present;
/// - otherwise a title-page front-matter heuristic is used — a short,
///   period-free first paragraph directly followed by a "by …" author line
///   becomes the book title (and is removed from the first chapter body);
/// - the filename stem is the final fallback. A generated "Chapter N"
///   placeholder or a first-heading text ("Chapter One", "Prologue", …) is
///   never used as the book title.
use std::io::Read;
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::{OebBook, OebChapter};
use crate::conversion::utils;

/// Parse a DOCX file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    let mut book =
        common::block_on(async move { crate::conversion::docx::parse(&path_buf).await })??;

    // (a) docProps/core.xml metadata — most authoritative.
    let (core_title, core_creator) = read_core_metadata(path);

    // (b) Title-page front-matter heuristic — only when the legacy pipeline
    //     left the generated "Chapter 1" placeholder (i.e. body content
    //     preceded the first heading).
    let front_matter = if core_title.is_none() {
        detect_front_matter(&book.chapters)
    } else {
        None
    };

    if let Some(t) = core_title {
        book.title = t;
    } else if let Some(fm) = &front_matter {
        book.title = fm.title.clone();
        if let Some(ch0) = book.chapters.first_mut() {
            ch0.html = strip_front_matter(&ch0.html, &fm.line1, fm.line2.as_deref());
        }
    } else if bad_inferred_title(&book.title) {
        // (c) Never keep a generated placeholder or a chapter heading as title.
        book.title = filename_stem(path);
    }

    if let Some(c) = core_creator {
        if book.authors.is_empty() {
            book.authors = vec![c];
        }
    }
    if book.authors.is_empty() {
        if let Some(a) = front_matter.as_ref().and_then(|fm| fm.author.clone()) {
            book.authors.push(a);
        }
    }

    // A surviving placeholder first chapter (heading-less book) is retitled
    // with the book title so the TOC does not show a generated label.
    if let Some(ch0) = book.chapters.first_mut() {
        if !ch0.html.trim().is_empty()
            && placeholder_title(ch0.title.as_deref())
            && !body_starts_with_heading(&ch0.html)
        {
            ch0.title = Some(book.title.clone());
        }
    }

    finalize_chapters(&mut book);
    Ok(book)
}

// ──────────────────────────────────────────────────────────────────────────
// TITLE / FRONT-MATTER POST-PROCESSING
// ──────────────────────────────────────────────────────────────────────────

/// Front matter detected at the start of the book: a title line/paragraph and
/// an optional "by …" author line, together with the exact HTML lines to
/// remove from the first chapter body.
struct FrontMatter {
    title: String,
    author: Option<String>,
    line1: String,
    line2: Option<String>,
}

/// Detect a title-page block in the first chapter. Only fires when the legacy
/// pipeline left a generated placeholder title ("Chapter 1" / "Full Text"),
/// i.e. body content preceded the first heading.
fn detect_front_matter(chapters: &[OebChapter]) -> Option<FrontMatter> {
    let ch0 = chapters.first()?;
    if !placeholder_title(ch0.title.as_deref()) {
        return None;
    }
    if body_starts_with_heading(&ch0.html) {
        return None;
    }
    let paras = first_paragraphs(&ch0.html);
    let (p1, l1) = paras.first()?;
    if !title_candidate(p1) {
        return None;
    }
    let (p2, l2) = paras.get(1)?;
    let author = author_from_by_line(p2)?;
    Some(FrontMatter {
        title: p1.clone(),
        author: Some(author),
        line1: l1.clone(),
        line2: Some(l2.clone()),
    })
}

/// A title candidate: short (≤ 60 chars), no sentence-ending punctuation,
/// and not a chapter/section heading (keyword, all-caps or numbered).
fn title_candidate(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty()
        && t.chars().count() <= 60
        && !(t.ends_with('.')
            || t.ends_with('!')
            || t.ends_with('?')
            || t.ends_with(':')
            || t.ends_with(';')
            || t.ends_with('…'))
        && !utils::looks_like_heading(t)
}

/// "by Test Author" → Some("Test Author") (case-insensitive "by " prefix).
fn author_from_by_line(text: &str) -> Option<String> {
    let t = text.trim();
    if t.get(..3).map_or(true, |p| !p.eq_ignore_ascii_case("by ")) {
        return None;
    }
    let author = t[3..].trim();
    if author.is_empty() {
        None
    } else {
        Some(author.to_string())
    }
}

/// First non-empty `<p>` paragraphs of a chapter body as (plain text, exact
/// trimmed HTML line). Heading elements are skipped.
fn first_paragraphs(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in html.lines() {
        let t = line.trim();
        if t.starts_with("<h1")
            || t.starts_with("<h2")
            || t.starts_with("<h3")
            || t.starts_with("<h4")
            || t.starts_with("<h5")
            || t.starts_with("<h6")
        {
            continue;
        }
        if let Some(inner) = t.strip_prefix("<p>").and_then(|s| s.strip_suffix("</p>")) {
            let plain = utils::strip_html_tags(inner).trim().to_string();
            if !plain.is_empty() {
                out.push((plain, t.to_string()));
            }
        }
    }
    out
}

/// Remove the two front-matter lines from the leading paragraphs of a chapter
/// body. Later occurrences of the same text are kept.
fn strip_front_matter(html: &str, line1: &str, line2: Option<&str>) -> String {
    let mut pending: Option<&str> = Some(line1);
    let mut out = String::new();
    for line in html.lines() {
        let t = line.trim();
        let drop = match pending {
            Some(want) if !t.is_empty() && t == want => {
                // Advance: line1 → line2 → done (line1 == line2 → done).
                pending = if want == line1 && line2.is_some() && line2 != Some(line1) {
                    line2
                } else {
                    None
                };
                true
            }
            Some(_) if !t.is_empty() => {
                // First non-front-matter paragraph reached — stop stripping.
                pending = None;
                false
            }
            _ => false,
        };
        if !drop {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// True for generated placeholder chapter titles ("Chapter 1", "Full Text",
/// "Document", "Untitled") that never make good book titles.
fn placeholder_title(title: Option<&str>) -> bool {
    let Some(t) = title else {
        return true;
    };
    let lower = t.trim().to_lowercase();
    if matches!(lower.as_str(), "full text" | "document" | "untitled") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("chapter ") {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

/// Whether a chapter body opens with a real (authored) heading element.
fn body_starts_with_heading(html: &str) -> bool {
    let t = html.trim_start();
    t.starts_with("<h1")
        || t.starts_with("<h2")
        || t.starts_with("<h3")
        || t.starts_with("<h4")
        || t.starts_with("<h5")
        || t.starts_with("<h6")
}

/// Section keywords that mark a heading as a chapter/front-matter heading
/// rather than a book title (mirrors `utils::looks_like_heading`).
const SECTION_KEYWORDS: &[&str] = &[
    "chapter",
    "part",
    "book",
    "prologue",
    "epilogue",
    "introduction",
    "preface",
    "afterword",
    "appendix",
    "interlude",
    "act",
    "scene",
    "volume",
    "section",
];

/// A legacy-inferred title is unusable when it is a generated placeholder or
/// reads like a section heading ("Chapter One", "Part II", "Prologue", …).
fn bad_inferred_title(title: &str) -> bool {
    let t = title.trim();
    if placeholder_title(Some(t)) {
        return true;
    }
    let lower = t.to_lowercase();
    SECTION_KEYWORDS.iter().any(|kw| {
        if lower == *kw {
            return true;
        }
        match lower.strip_prefix(kw) {
            Some(rest) => rest.chars().next().map_or(false, |c| c.is_whitespace()),
            None => false,
        }
    })
}

/// Fallback book title from the source file name.
fn filename_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace('_', " ").replace('-', " "))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled".to_string())
}

/// Drop empty chapters, renumber ids sequentially, and guarantee at least
/// one chapter exists.
fn finalize_chapters(book: &mut OebBook) {
    book.chapters.retain(|ch| !ch.html.trim().is_empty());
    for (i, ch) in book.chapters.iter_mut().enumerate() {
        ch.id = format!("chapter_{:03}", i + 1);
    }
    if book.chapters.is_empty() {
        book.chapters.push(OebChapter {
            id: "chapter_001".to_string(),
            title: Some(book.title.clone()),
            html: "<p>&#160;</p>".to_string(),
        });
    }
}

// ──────────────────────────────────────────────────────────────────────────
// DOCX CORE METADATA (docProps/core.xml)
// ──────────────────────────────────────────────────────────────────────────

/// Read `docProps/core.xml` metadata — `dc:title` and `dc:creator` — when
/// present. Missing files / malformed XML yield `(None, None)`.
fn read_core_metadata(path: &Path) -> (Option<String>, Option<String>) {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return (None, None),
    };
    let cursor = std::io::Cursor::new(&data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return (None, None),
    };
    let mut core = match archive.by_name("docProps/core.xml") {
        Ok(f) => f,
        Err(_) => return (None, None),
    };
    let mut xml = String::new();
    if core.read_to_string(&mut xml).is_err() {
        return (None, None);
    }

    let mut title = None;
    let mut creator = None;
    let mut current: Option<&'static str> = None;
    let mut reader = quick_xml::Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => {
                let name = local_name(e.name().as_ref());
                current = match name.as_str() {
                    "title" => Some("title"),
                    "creator" => Some("creator"),
                    _ => None,
                };
            }
            Ok(quick_xml::events::Event::Text(e)) => {
                if let Some(kind) = current {
                    let text = e.unescape().unwrap_or_default().trim().to_string();
                    if !text.is_empty() {
                        if kind == "title" && title.is_none() {
                            title = Some(text);
                        } else if kind == "creator" && creator.is_none() {
                            creator = Some(text);
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::End(e)) => {
                let name = local_name(e.name().as_ref());
                if name == "title" || name == "creator" {
                    current = None;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    (title, creator)
}

fn local_name(name: &[u8]) -> String {
    let full = String::from_utf8_lossy(name).to_string();
    full.rsplit_once(':')
        .map(|(_, n)| n.to_string())
        .unwrap_or(full)
}
