//! `reading entries export`, which prints the file of a list as it is.

use std::fs;
use std::path::Path;

use crumbkit::{Bytes, Document};

use super::{EntriesError, file};

/// The document that `reading entries export` prints: the bytes of a reading
/// list's file.
pub(crate) const ENTRIES_EXPORT: Document<Bytes> = Document::new(
    "reading",
    "entries-export",
    1,
    "The bytes of a reading list's file, as the file holds them.",
);

/// Returns the bytes of the file of the list named `name` in `dir`,
/// including its about line, its comments and its blank lines.
///
/// # Errors
///
/// Returns `EntriesError::NotFound` when `dir` is `None` or holds no list
/// named `name`, and `EntriesError::CannotOpen` with the file's path when
/// the list's file cannot be read.
pub(crate) fn export(dir: Option<&Path>, name: &str) -> Result<Bytes, EntriesError> {
    let path = file(dir, name)?;
    let bytes = fs::read(&path).map_err(|error| EntriesError::CannotOpen {
        name: name.to_owned(),
        path: path.clone(),
        error,
    })?;
    Ok(Bytes::new(bytes))
}
