/// DOCX → OEB parser.
///
/// Delegates to the legacy `crate::conversion::docx::parse` pipeline, which
/// walks `word/document.xml` paragraph-by-paragraph with style inheritance,
/// list state, image extraction via relationships, footnotes and page-break
/// chapter boundaries.
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::OebBook;

/// Parse a DOCX file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    common::block_on(async move { crate::conversion::docx::parse(&path_buf).await })?
}
