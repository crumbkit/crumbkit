//! The AGENTS.md word check: an `AGENTS.md` file may have at most 200 words.

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of whitespace-separated words an `AGENTS.md` may have.
const LIMIT: usize = 200;

/// Reports each `AGENTS.md` file in `files`, in any directory, that has more
/// than 200 words.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| Path::new(&file.path).file_name() == Some(OsStr::new("AGENTS.md")))
        .map(|file| Violation {
            path: file.path.clone(),
            measured: file.contents.split_whitespace().count(),
            unit: "words",
            limit: LIMIT,
        })
        .filter(|violation| violation.measured > LIMIT)
        .collect()
}
