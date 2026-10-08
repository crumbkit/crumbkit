//! Tests for the Rust file length check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, lines: usize) -> File {
    File {
        path: path.to_owned(),
        contents: "let x = 1;\n".repeat(lines),
    }
}

#[test]
fn a_rust_file_of_exactly_600_lines_passes() {
    assert_eq!(check(&[file("src/lib.rs", 600)]), []);
}

#[test]
fn a_rust_file_of_601_lines_is_reported_with_its_count_and_the_limit() {
    assert_eq!(
        check(&[file("src/lib.rs", 601)]),
        [Violation::OverLimit {
            path: "src/lib.rs".to_owned(),
            measured: 601,
            unit: "lines",
            limit: 600,
        }]
    );
}

#[test]
fn a_long_file_that_is_not_rust_is_ignored() {
    assert_eq!(check(&[file("docs/notes.md", 601)]), []);
}
