//! `reading where`, which prints the directory that holds reading's lists.

use std::fmt;
use std::path::Path;

use crumbkit::{Document, Prose};
use schemars::JsonSchema;
use serde::Serialize;

#[cfg(test)]
mod tests;

/// The document that `reading where` prints: the directory that holds
/// reading's lists.
pub(crate) const WHERE: Document<Prose<Location>> = Document::new(
    "reading",
    "where",
    1,
    "The directory that holds reading's lists.",
);

/// The directory that holds reading's lists.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Location {
    /// The directory, as `READING_DIR` names it or under the XDG data
    /// directory, or null when `READING_DIR` is unset and the home
    /// directory is unknown.
    dir: Option<String>,
}

impl Location {
    /// Returns the location of `dir`, the directory of reading's lists, or
    /// of no directory.
    pub(crate) fn of(dir: Option<&Path>) -> Self {
        Self {
            dir: dir.map(|dir| dir.display().to_string()),
        }
    }
}

/// Prints the directory alone, so a script can read it, or one sentence that
/// says why reading cannot find it.
impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.dir {
            Some(dir) => f.write_str(dir),
            None => f.write_str(
                "reading cannot find its lists, because READING_DIR is unset and the home directory is unknown.",
            ),
        }
    }
}
