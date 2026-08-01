/// MOBI / AZW3 → OEB parser.
///
/// Delegates to the legacy `crate::conversion::mobi::parse` pipeline, which
/// implements full PalmDB/MOBI parsing: LZ77 + HuffDic decompression, EXTH
/// metadata, KF8 (AZW3) detection, image extraction and HTML chapter
/// splitting.
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::OebBook;

/// Parse a MOBI/AZW3 file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    common::block_on(async move { crate::conversion::mobi::parse(&path_buf).await })?
}
