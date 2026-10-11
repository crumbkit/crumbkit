//! The README word check: a `README.md` outside the `docs/` tree may have at
//! most 650 words.

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of whitespace-separated words that a README may have.
const LIMIT: usize = 650;

/// Reports each `README.md` outside the `docs/` tree that has more than 650
/// words. The count covers the whole file, including code blocks and link
/// definitions. The READMEs inside `docs/` are node records, which the docs
/// word check counts.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| {
            let path = Path::new(&file.path);
            path.file_name() == Some(OsStr::new("README.md")) && !path.starts_with("docs")
        })
        .filter_map(|file| {
            let measured = file.contents.split_whitespace().count();
            (measured > LIMIT).then(|| Violation::OverLimit {
                path: file.path.clone(),
                measured,
                unit: "words",
                limit: LIMIT,
            })
        })
        .collect()
}
