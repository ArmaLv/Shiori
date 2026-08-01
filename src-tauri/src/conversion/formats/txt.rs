/// TXT / RTF → OEB parser.
///
/// Delegates to the legacy `crate::conversion::txt::parse` pipeline, which
/// detects paragraph mode (markdown / formatted / unformatted), handles
/// encoding detection and splits chapters with heading heuristics.
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::OebBook;

/// Parse a TXT/RTF file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    common::block_on(async move { crate::conversion::txt::parse(&path_buf).await })?
}
