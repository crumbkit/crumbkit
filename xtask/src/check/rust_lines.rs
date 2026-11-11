//! The Rust file length check: a `.rs` file may have at most 600 code lines.

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The largest number of code lines a `.rs` file may have.
const LIMIT: usize = 600;

/// Reports each `.rs` file in `files` that has more than 600 code lines.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    files
        .iter()
        .filter(|file| Path::new(&file.path).extension() == Some(OsStr::new("rs")))
        .filter_map(|file| {
            let measured = code_lines(&file.contents);
            (measured > LIMIT).then(|| Violation::OverLimit {
                path: file.path.clone(),
                measured,
                unit: "code lines",
                limit: LIMIT,
            })
        })
        .collect()
}

/// Counts the code lines of `contents`: the lines that hold a character
/// other than whitespace and whose first such characters are not `//`.
///
/// This function reads the start of each line and does not parse Rust
/// (RUST-10). It handles blank lines, line comments and doc comments. It
/// counts every line of a block comment (`/* ... */`) as code, and it skips
/// a line of a multi-line string literal that starts with `//`. No `.rs`
/// file in the repository held either case when this was written. A
/// tokenizer such as `rustc_lexer` or `syn` replaces this function once
/// either case appears in the tree.
fn code_lines(contents: &str) -> usize {
    contents
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .count()
}
