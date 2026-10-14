//! Tests for the docs trailer check.

use super::check;
use crate::check::{Commit, File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

fn changelog(entries: &str) -> File {
    file(
        "CHANGELOG.md",
        &format!(
            "# Changelog\n\n## [Unreleased]\n\n### Added\n\n{entries}\n\
             ## [0.1.0] - 2026-10-01\n\n- An old entry.\n"
        ),
    )
}

fn commit(message: &str) -> Commit {
    Commit {
        id: "abc1234".to_owned(),
        message: message.to_owned(),
    }
}

const OWED: &str = "Docs-Owed: json-layout; from: one line; to: indented; \
                    docs: docs/behaviour/output/json-layout.md, crates/crumbkit/src/output.rs";

#[test]
fn a_change_without_a_new_changelog_entry_needs_no_trailer() {
    let base = [changelog("- An entry.\n")];
    let head = [changelog("- An entry.\n")];

    assert_eq!(check(&base, &head, &[commit("A fix")]), []);
}

#[test]
fn a_new_unreleased_entry_without_a_trailer_is_reported() {
    let base = [changelog("- An entry.\n")];
    let head = [changelog("- An entry.\n- A new entry.\n")];

    assert_eq!(
        check(&base, &head, &[commit("Tools print JSON indented")]),
        [Violation::MissingDocsTrailer {
            path: "CHANGELOG.md".to_owned(),
        }]
    );
}

#[test]
fn a_changed_unreleased_entry_without_a_trailer_is_reported() {
    let base = [changelog("- An entry.\n")];
    let head = [changelog("- An entry that\n  changed.\n")];

    assert_eq!(
        check(&base, &head, &[commit("Reword")]),
        [Violation::MissingDocsTrailer {
            path: "CHANGELOG.md".to_owned(),
        }]
    );
}

#[test]
fn a_new_entry_with_a_docs_owed_trailer_passes() {
    let base = [changelog("")];
    let head = [changelog("- A new entry.\n")];
    let message = format!("Tools print JSON indented\n\nThe body.\n\n{OWED}\n");

    assert_eq!(check(&base, &head, &[commit(&message)]), []);
}

#[test]
fn a_new_entry_with_a_docs_swept_trailer_in_any_commit_passes() {
    let base = [changelog("")];
    let head = [changelog("- A new entry.\n")];
    let commits = [
        commit("Tools print JSON indented"),
        commit("The docs describe indented JSON\n\nDocs-Swept: json-layout, help-pages\n"),
    ];

    assert_eq!(check(&base, &head, &commits), []);
}

#[test]
fn entries_that_a_release_moves_out_of_unreleased_need_no_trailer() {
    let base = [changelog("- An entry.\n")];
    let head = [changelog("")];

    assert_eq!(check(&base, &head, &[commit("Release 0.2.0")]), []);
}

#[test]
fn a_trailer_outside_the_last_paragraph_does_not_count() {
    let base = [changelog("")];
    let head = [changelog("- A new entry.\n")];
    let message = "Subject\n\nDocs-Swept: json-layout\n\nA closing paragraph.\n";

    assert_eq!(
        check(&base, &head, &[commit(message)]),
        [Violation::MissingDocsTrailer {
            path: "CHANGELOG.md".to_owned(),
        }]
    );
}

#[test]
fn a_malformed_trailer_is_reported_even_without_a_changelog_change() {
    let malformed = [
        "Docs-Swept: Json_Layout",
        "Docs-Swept:",
        "Docs-Owed: json-layout; from: one line; to: indented",
        "Docs-Owed: json-layout; from: ; to: indented; docs: docs/README.md",
        "Docs-Owed: json-layout; from: a; to: b; docs: docs/README.md, , docs/a.md",
        "Docs-Owed: -json; from: a; to: b; docs: docs/README.md",
    ];
    for trailer in malformed {
        let message = format!("Subject\n\n{trailer}\n");

        assert_eq!(
            check(&[], &[], &[commit(&message)]),
            [Violation::MalformedTrailer {
                commit: "abc1234".to_owned(),
                trailer: trailer.to_owned(),
            }],
            "{trailer}"
        );
    }
}

#[test]
fn a_malformed_trailer_does_not_satisfy_a_changelog_change() {
    let base = [changelog("")];
    let head = [changelog("- A new entry.\n")];

    assert_eq!(
        check(&base, &head, &[commit("Subject\n\nDocs-Swept: Bad Id\n")]),
        [
            Violation::MalformedTrailer {
                commit: "abc1234".to_owned(),
                trailer: "Docs-Swept: Bad Id".to_owned(),
            },
            Violation::MissingDocsTrailer {
                path: "CHANGELOG.md".to_owned(),
            },
        ]
    );
}

#[test]
fn a_changelog_in_a_crate_directory_is_checked_too() {
    let base = [file("crates/a/CHANGELOG.md", "## [Unreleased]\n\n")];
    let head = [file("crates/a/CHANGELOG.md", "## [Unreleased]\n\n- New.\n")];

    assert_eq!(
        check(&base, &head, &[commit("Subject")]),
        [Violation::MissingDocsTrailer {
            path: "crates/a/CHANGELOG.md".to_owned(),
        }]
    );
}

#[test]
fn a_branch_without_commits_needs_no_trailer() {
    let head = [changelog("- An entry.\n")];

    assert_eq!(check(&[], &head, &[]), []);
}
