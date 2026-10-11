//! The changelog entry check: an entry in a `CHANGELOG.md` may have at most
//! 40 words.

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of whitespace-separated words that one changelog entry
/// may have.
const LIMIT: usize = 40;

/// Reports each changelog entry that has more than 40 words, by the
/// changelog's path and the line the entry starts on. An entry is a line that
/// starts with `- ` and the indented lines that follow it.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| Path::new(&file.path).file_name() == Some(OsStr::new("CHANGELOG.md")))
        .flat_map(|file| {
            entries(&file.contents)
                .into_iter()
                .filter(|&(_, words)| words > LIMIT)
                .map(|(line, words)| Violation::OverLimit {
                    path: format!("{}:{line}", file.path),
                    measured: words,
                    unit: "words in this entry",
                    limit: LIMIT,
                })
        })
        .collect()
}

/// Returns the line number and word count of each entry in `contents`.
fn entries(contents: &str) -> Vec<(usize, usize)> {
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut in_entry = false;
    for (index, line) in contents.lines().enumerate() {
        if let Some(text) = line.strip_prefix("- ") {
            entries.push((index + 1, text.split_whitespace().count()));
            in_entry = true;
        } else if in_entry && line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some((_, words)) = entries.last_mut() {
                *words += line.split_whitespace().count();
            }
        } else {
            in_entry = false;
        }
    }
    entries
}
