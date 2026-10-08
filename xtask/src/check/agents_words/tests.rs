//! Tests for the AGENTS.md word check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, words: usize) -> File {
    File {
        path: path.to_owned(),
        contents: "word ".repeat(words),
    }
}

#[test]
fn an_agents_file_of_exactly_400_words_passes() {
    assert_eq!(check(&[file("AGENTS.md", 400)]), []);
}

#[test]
fn an_agents_file_of_401_words_is_reported_with_its_count_and_the_limit() {
    assert_eq!(
        check(&[file("AGENTS.md", 401)]),
        [Violation {
            path: "AGENTS.md".to_owned(),
            measured: 401,
            unit: "words",
            limit: 400,
        }]
    );
}

#[test]
fn an_agents_file_in_a_subdirectory_is_checked() {
    assert_eq!(check(&[file("crates/crumbkit/AGENTS.md", 401)]).len(), 1);
}

#[test]
fn words_are_separated_by_any_whitespace() {
    let agents = File {
        path: "AGENTS.md".to_owned(),
        contents: "one\ttwo\n\nthree   four\n".repeat(101),
    };

    assert_eq!(check(&[agents])[0].measured, 404);
}

#[test]
fn other_markdown_files_are_ignored() {
    assert_eq!(
        check(&[file("README.md", 401), file("NOT_AGENTS.md", 401)]),
        []
    );
}
