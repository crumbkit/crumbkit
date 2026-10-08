//! The property test configuration check: every `proptest!` block starts with
//! `#![proptest_config(crate::property::config())]`, which fixes the seed and
//! turns off failure persistence (RUST-11, ADR-0006).

use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The line that must follow the first line of each `proptest!` block.
const CONFIG_LINE: &str = "#![proptest_config(crate::property::config())]";

/// Reports each `proptest!` block in a `.rs` file of `files` whose next line
/// is not [`CONFIG_LINE`], with the number of the block's first line.
///
/// RUST-10: this check reads lines of text. It finds a block whose first line
/// starts with `proptest!` or `proptest::proptest!` after any indentation,
/// which is how crumbkit writes them, and rustfmt leaves the inside of the
/// block as written. It misses a block that calls the macro under another
/// name, and it reports a block that has the attribute on its first line or
/// after a comment. A parser such as syn would find every invocation and read
/// the attribute wherever it is; use one if such a block is ever needed.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in files {
        if Path::new(&file.path).extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let lines: Vec<&str> = file.contents.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let line = line.trim_start();
            if !(line.starts_with("proptest!") || line.starts_with("proptest::proptest!")) {
                continue;
            }
            let next = lines.get(index + 1).map(|next| next.trim());
            if next != Some(CONFIG_LINE) {
                violations.push(Violation::UnconfiguredPropertyTests {
                    path: file.path.clone(),
                    line: index + 1,
                });
            }
        }
    }
    violations
}
