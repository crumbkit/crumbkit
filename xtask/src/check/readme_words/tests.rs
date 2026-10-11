//! Tests for the README word check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, words: usize) -> File {
    File {
        path: path.to_owned(),
        contents: "word ".repeat(words),
    }
}

#[test]
fn a_readme_of_exactly_650_words_passes() {
    let files = [file("README.md", 650)];

    assert_eq!(check(&files), []);
}

#[test]
fn a_readme_of_651_words_is_reported_with_its_count_and_the_limit() {
    let files = [file("crates/crumbkit/README.md", 651)];

    assert_eq!(
        check(&files),
        [Violation::OverLimit {
            path: "crates/crumbkit/README.md".to_owned(),
            measured: 651,
            unit: "words",
            limit: 650,
        }]
    );
}

#[test]
fn each_readme_is_counted_on_its_own() {
    let files = [
        file("README.md", 600),
        file("crates/crumbkit/README.md", 600),
        file("crates/crumbkit-test/README.md", 600),
    ];

    assert_eq!(check(&files), []);
}

#[test]
fn every_word_in_the_file_counts_including_code_and_link_definitions() {
    let contents = format!(
        "{}\n```rust\nlet a = 1;\n```\n[a]: https://example.com\n",
        "word ".repeat(643)
    );
    let files = [File {
        path: "README.md".to_owned(),
        contents,
    }];

    assert!(matches!(
        &check(&files)[..],
        [Violation::OverLimit { measured: 651, .. }]
    ));
}

#[test]
fn readmes_in_the_docs_tree_and_other_markdown_files_are_ignored() {
    let files = [
        file("docs/README.md", 651),
        file("docs/decisions/README.md", 651),
        file("CHANGELOG.md", 651),
        file("crates/crumbkit/notes.md", 651),
    ];

    assert_eq!(check(&files), []);
}
