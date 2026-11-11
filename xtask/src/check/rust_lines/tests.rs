//! Tests for the Rust file length check.

use super::check;
use crate::check::{File, Violation};

/// Builds a file of `code` lines of code, then `comments` line comments,
/// then `blank` empty lines.
fn file(path: &str, code: usize, comments: usize, blank: usize) -> File {
    let mut contents = "let x = 1;\n".repeat(code);
    contents.push_str(&"// A note.\n".repeat(comments));
    contents.push_str(&"\n".repeat(blank));
    File {
        path: path.to_owned(),
        contents,
    }
}

/// Builds `src/lib.rs` with 600 lines of code followed by `rest`.
fn full_file_and(rest: &str) -> File {
    let mut file = file("src/lib.rs", 600, 0, 0);
    file.contents.push_str(rest);
    file
}

/// The violation that reports `src/lib.rs` with `measured` code lines.
fn over_limit(measured: usize) -> Violation {
    Violation::OverLimit {
        path: "src/lib.rs".to_owned(),
        measured,
        unit: "code lines",
        limit: 600,
    }
}

#[test]
fn a_rust_file_of_exactly_600_code_lines_passes() {
    assert_eq!(check(&[file("src/lib.rs", 600, 0, 0)]), []);
}

#[test]
fn a_rust_file_of_601_code_lines_is_reported_with_its_count_of_code_lines() {
    assert_eq!(check(&[file("src/lib.rs", 601, 0, 0)]), [over_limit(601)]);
}

#[test]
fn a_rust_file_of_600_code_lines_and_400_comment_lines_passes() {
    assert_eq!(check(&[file("src/lib.rs", 600, 400, 0)]), []);
}

#[test]
fn blank_lines_and_lines_of_only_whitespace_are_not_counted() {
    assert_eq!(check(&[file("src/lib.rs", 600, 0, 50)]), []);
    assert_eq!(check(&[full_file_and("    \n\t\n \t \n")]), []);
}

#[test]
fn line_comments_and_doc_comments_are_not_counted_at_any_indent() {
    let comments = concat!(
        "// A note.\n",
        "/// An outer doc.\n",
        "//! An inner doc.\n",
        "    // An indented note.\n",
        "\t\t/// An indented outer doc.\n",
        "        //! An indented inner doc.\n",
        "//\n",
    );
    assert_eq!(check(&[full_file_and(comments)]), []);
}

#[test]
fn a_line_of_code_with_a_trailing_comment_is_counted() {
    assert_eq!(
        check(&[full_file_and("let y = 2; // A trailing note.\n")]),
        [over_limit(601)]
    );
}

#[test]
fn the_lines_of_a_block_comment_are_counted() {
    assert_eq!(
        check(&[full_file_and("/*\n A block comment.\n*/\n")]),
        [over_limit(603)]
    );
}

#[test]
fn a_long_file_that_is_not_rust_is_ignored() {
    assert_eq!(check(&[file("docs/notes.md", 601, 0, 0)]), []);
}
