//! Tests for the changelog entry word check.

use super::check;
use crate::check::{File, Violation};

fn changelog(contents: &str) -> File {
    File {
        path: "CHANGELOG.md".to_owned(),
        contents: contents.to_owned(),
    }
}

/// An entry of `words` words, wrapped over lines of ten words as the
/// changelog wraps them.
fn entry(words: usize) -> String {
    let all: Vec<&str> = std::iter::repeat_n("word", words).collect();
    let lines: Vec<String> = all.chunks(10).map(|line| line.join(" ")).collect();
    format!("- {}\n", lines.join("\n  "))
}

#[test]
fn an_entry_of_exactly_40_words_passes() {
    let files = [changelog(&format!("## [Unreleased]\n\n{}", entry(40)))];

    assert_eq!(check(&files), []);
}

#[test]
fn an_entry_of_41_words_is_reported_by_its_line_with_its_count_and_the_limit() {
    let contents = format!("# Changelog\n\n### Added\n\n{}{}", entry(5), entry(41));

    assert_eq!(
        check(&[changelog(&contents)]),
        [Violation::OverLimit {
            path: "CHANGELOG.md:6".to_owned(),
            measured: 41,
            unit: "words in this entry",
            limit: 40,
        }]
    );
}

#[test]
fn an_entry_counts_its_indented_continuation_lines_and_stops_at_a_blank_line() {
    let contents = format!("{}\nword word word word word\n", entry(38));

    assert_eq!(check(&[changelog(&contents)]), []);
}

#[test]
fn headings_comments_and_link_definitions_are_not_entries() {
    let long = "word ".repeat(41);
    let contents = format!(
        "# {long}\n\n<!-- {long} -->\n\n[a]: https://example.com/{}\n",
        long.replace(' ', "/")
    );

    assert_eq!(check(&[changelog(&contents)]), []);
}

#[test]
fn files_other_than_a_changelog_are_ignored() {
    let files = [File {
        path: "README.md".to_owned(),
        contents: entry(41),
    }];

    assert_eq!(check(&files), []);
}
