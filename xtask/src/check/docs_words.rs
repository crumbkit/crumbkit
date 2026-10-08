//! The docs word check: the Markdown files directly inside each directory of
//! the `docs/` tree may total at most 5,000 words.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of whitespace-separated words that the Markdown files
/// directly inside one docs directory may hold together.
const LIMIT: usize = 5000;

/// Reports each directory in the `docs/` tree whose own Markdown files total
/// more than 5,000 words. Files in a subdirectory count only towards that
/// subdirectory.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    let mut words_by_directory: BTreeMap<&str, usize> = BTreeMap::new();
    for file in files {
        let path = Path::new(&file.path);
        if !path.starts_with("docs") || path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        let Some(directory) = path.parent().and_then(Path::to_str) else {
            continue;
        };
        *words_by_directory.entry(directory).or_default() +=
            file.contents.split_whitespace().count();
    }

    words_by_directory
        .into_iter()
        .filter(|&(_, words)| words > LIMIT)
        .map(|(directory, words)| Violation::OverLimit {
            path: directory.to_owned(),
            measured: words,
            unit: "words in this directory's Markdown",
            limit: LIMIT,
        })
        .collect()
}
