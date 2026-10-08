//! The Rust file length check: a `.rs` file may have at most 600 lines.

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of lines a `.rs` file may have.
const LIMIT: usize = 600;

/// Reports each `.rs` file in `files` that has more than 600 lines.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| Path::new(&file.path).extension() == Some(OsStr::new("rs")))
        .filter_map(|file| {
            let measured = file.contents.lines().count();
            (measured > LIMIT).then(|| Violation::OverLimit {
                path: file.path.clone(),
                measured,
                unit: "lines",
                limit: LIMIT,
            })
        })
        .collect()
}
