/// FB2 / FBZ → OEB parser.
///
/// Delegates to the legacy `crate::conversion::fb2::parse` pipeline, which
/// handles gzip/zip-compressed FB2, metadata, base64 binary images, the
/// section tree and the notes body.
use std::path::Path;

use crate::conversion::error::ConversionError;
use crate::conversion::formats::common;
use crate::conversion::oeb::OebBook;

/// Parse an FB2/FBZ file into an OebBook.
pub fn parse(path: &Path) -> Result<OebBook, ConversionError> {
    let path_buf = path.to_path_buf();
    common::block_on(async move { crate::conversion::fb2::parse(&path_buf).await })?
}
