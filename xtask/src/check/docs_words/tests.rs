//! Tests for the docs directory word check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, words: usize) -> File {
    File {
        path: path.to_owned(),
        contents: "word ".repeat(words),
    }
}

#[test]
fn a_docs_directory_of_exactly_5000_words_passes() {
    let files = [file("docs/a.md", 3000), file("docs/b.md", 2000)];

    assert_eq!(check(&files), []);
}

#[test]
fn a_docs_directory_of_5001_words_is_reported_with_its_total_and_the_limit() {
    let files = [file("docs/a.md", 3000), file("docs/b.md", 2001)];

    assert_eq!(
        check(&files),
        [Violation::OverLimit {
            path: "docs".to_owned(),
            measured: 5001,
            unit: "words in this directory's Markdown",
            limit: 5000,
        }]
    );
}

#[test]
fn each_directory_in_the_docs_tree_is_counted_on_its_own() {
    let files = [
        file("docs/a.md", 4000),
        file("docs/decisions/a.md", 4000),
        file("docs/decisions/old/a.md", 4000),
    ];

    assert_eq!(check(&files), []);
}

#[test]
fn a_subdirectory_over_the_limit_is_reported_by_its_path() {
    let files = [file("docs/standards/a.md", 5001)];

    assert!(matches!(
        &check(&files)[..],
        [Violation::OverLimit { path, .. }] if path == "docs/standards"
    ));
}

#[test]
fn files_outside_the_docs_tree_and_files_that_are_not_markdown_are_ignored() {
    let files = [
        file("README.md", 5001),
        file("crates/crumbkit/docs/a.md", 5001),
        file("docs/book.toml", 5001),
    ];

    assert_eq!(check(&files), []);
}
