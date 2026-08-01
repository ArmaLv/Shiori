/// Markdown → OEB parser.
///
/// Parses with pulldown-cmark and splits the document into chapters on
/// headings. The first `h1` is treated as the book title (mirroring
/// `MarkdownFormatAdapter::extract_first_heading`); subsequent headings
/// become chapters. Local images referenced by relative paths are embedded.
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::{OebBook, OebChapter, OebImage};
use crate::services::adapters::MarkdownFormatAdapter;
use crate::services::format_adapter::BookFormatAdapter;

/// Parse a Markdown file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let text = std::fs::read_to_string(path).map_err(ConversionError::IoError)?;

    // Metadata via the shared adapter (error-tolerant: filename fallback)
    let adapter = MarkdownFormatAdapter::new();
    let meta = common::block_on(adapter.extract_metadata(path))?.unwrap_or_default();

    let title = if !meta.title.is_empty() && meta.title != "Unknown" {
        meta.title
    } else {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string()
    };

    let mut book = OebBook::new(title);
    book.authors = meta.authors;
    book.language = meta.language.clone().unwrap_or_else(|| "en".to_string());
    book.description = meta.description;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut images: Vec<OebImage> = Vec::new();
    let mut img_counter = 0u32;

    let chapters = split_into_chapters(&text, base_dir, &mut images, &mut img_counter)?;

    for (i, (ch_title, ch_html)) in chapters.into_iter().enumerate() {
        book.chapters.push(OebChapter {
            id: format!("chapter_{:03}", i + 1),
            title: Some(ch_title),
            html: ch_html,
        });
    }

    book.images = images;
    Ok(book)
}

/// Render a markdown event stream into an XHTML string.
fn render<'a>(events: impl Iterator<Item = Event<'a>>) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    html
}

/// Split markdown into (title, xhtml) chapters, embedding local images.
///
/// Buffer layout:
/// - `preamble` — content before the first real chapter heading (merged into
///   the first chapter, so a title heading's intro paragraph is not lost).
/// - `current_html` — body of the chapter being accumulated.
fn split_into_chapters(
    text: &str,
    base_dir: &Path,
    images: &mut Vec<OebImage>,
    img_counter: &mut u32,
) -> Result<Vec<(String, String)>, ConversionError> {
    let mut parser = Parser::new_ext(text, Options::all());

    let mut preamble = String::new();
    let mut current_title: Option<String> = None;
    let mut current_html = String::new();
    let mut pending: Vec<Event> = Vec::new();
    let mut chapters: Vec<(String, String)> = Vec::new();
    let mut seen_chapter = false;

    let flush = |pending: &mut Vec<Event>, out: &mut String| {
        if !pending.is_empty() {
            pulldown_cmark::html::push_html(out, pending.drain(..));
        }
    };

    while let Some(ev) = parser.next() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                // Commit any pending content to the correct buffer.
                if seen_chapter {
                    flush(&mut pending, &mut current_html);
                } else {
                    flush(&mut pending, &mut preamble);
                }
                // Commit the previous chapter (if any).
                if let Some(t) = current_title.take() {
                    if !current_html.trim().is_empty() {
                        chapters.push((t, std::mem::take(&mut current_html)));
                    }
                }

                // Collect the heading's events (Start … End) to render it.
                let mut heading_events: Vec<Event> = vec![ev];
                let mut heading_text = String::new();
                loop {
                    match parser.next() {
                        Some(Event::End(TagEnd::Heading(_))) => {
                            heading_events.push(Event::End(TagEnd::Heading(level)));
                            break;
                        }
                        Some(Event::Text(t)) => heading_text.push_str(&t),
                        Some(other) => heading_events.push(other),
                        None => break,
                    }
                }
                let heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    _ => 4,
                };

                let is_first_h1 =
                    !seen_chapter && heading_level == 1 && !heading_text.trim().is_empty();
                if is_first_h1 {
                    // Consumed as the book title — the metadata title already
                    // equals this heading (adapter convention).
                    continue;
                }

                seen_chapter = true;
                let heading_html = render(heading_events.into_iter());
                let t = if heading_text.trim().is_empty() {
                    "Section".to_string()
                } else {
                    heading_text.trim().to_string()
                };

                // Merge the preamble into the first chapter so content between
                // the title heading and the first chapter heading survives.
                current_html = format!("{}{}", std::mem::take(&mut preamble), heading_html);
                current_title = Some(t);
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                // Consume the image's alt text and render an <img> ourselves so
                // local files can be embedded.
                let mut alt = String::new();
                loop {
                    match parser.next() {
                        Some(Event::Text(t)) => alt.push_str(&t),
                        Some(Event::End(TagEnd::Image)) => break,
                        Some(_) => {}
                        None => break,
                    }
                }
                let alt = escape_attr(&alt.trim());
                match common::embed_local_image(&dest_url, base_dir, images, img_counter) {
                    Some(internal) => {
                        pending.push(Event::Html(
                            format!("<img src=\"{}\" alt=\"{}\"/>", internal, alt).into(),
                        ));
                    }
                    None => {
                        // External / data: URI — keep a plain image reference.
                        pending.push(Event::Html(
                            format!("<img src=\"{}\" alt=\"{}\"/>", escape_attr(&dest_url), alt)
                                .into(),
                        ));
                    }
                }
            }
            other => pending.push(other),
        }
    }

    // Final flush.
    if seen_chapter {
        flush(&mut pending, &mut current_html);
    } else {
        flush(&mut pending, &mut preamble);
    }

    if let Some(t) = current_title.take() {
        if !current_html.trim().is_empty() {
            chapters.push((t, current_html));
        }
    } else if !preamble.trim().is_empty() || !current_html.trim().is_empty() {
        // No chapter headings at all — one chapter with everything.
        chapters.push((
            "Content".to_string(),
            format!("{}{}", preamble, current_html),
        ));
    }

    if chapters.is_empty() {
        return Err(ConversionError::EmptyContent);
    }

    Ok(chapters)
}

/// Escape a value for use inside an XML attribute.
fn escape_attr(value: &str) -> String {
    crate::conversion::oeb::escape_xml(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_chapters() {
        let md = "# My Book\n\nIntro paragraph.\n\n## Chapter One\n\nBody one.\n\n## Chapter Two\n\nBody two.";
        let (mut images, mut counter) = (Vec::new(), 0u32);
        let chapters = split_into_chapters(md, Path::new("."), &mut images, &mut counter).unwrap();

        // First h1 is the title — only the two h2s become chapters.
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].0, "Chapter One");
        assert!(chapters[0].1.contains("Intro paragraph."));
        assert!(chapters[0].1.contains("Body one."));
        assert_eq!(chapters[1].0, "Chapter Two");
        assert!(chapters[1].1.contains("Body two."));
    }

    #[test]
    fn test_split_no_headings() {
        let md = "Just some text.\n\nMore text.";
        let (mut images, mut counter) = (Vec::new(), 0u32);
        let chapters = split_into_chapters(md, Path::new("."), &mut images, &mut counter).unwrap();
        assert_eq!(chapters.len(), 1);
        assert!(chapters[0].1.contains("Just some text."));
    }
}
