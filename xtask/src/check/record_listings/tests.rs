//! Tests for the record listing check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

/// A node record whose Children section links to each of `children`.
fn node(path: &str, children: &[&str]) -> File {
    let links: Vec<String> = children
        .iter()
        .map(|child| format!("- [A child]({child})"))
        .collect();
    file(
        path,
        &format!("# A node\n\n## Children\n\n{}\n", links.join("\n")),
    )
}

fn record(path: &str) -> File {
    file(path, "# A record\n")
}

#[test]
fn a_tree_whose_nodes_list_every_child_passes() {
    let files = [
        node("docs/decisions/README.md", &["api/README.md"]),
        node("docs/decisions/api/README.md", &["0001-errors.md"]),
        record("docs/decisions/api/0001-errors.md"),
    ];

    assert_eq!(check(&files), []);
}

#[test]
fn a_record_its_directory_readme_does_not_list_is_reported() {
    let files = [
        node("docs/decisions/README.md", &["0001-errors.md"]),
        record("docs/decisions/0001-errors.md"),
        record("docs/decisions/0002-api.md"),
    ];

    assert_eq!(
        check(&files),
        [Violation::UnlistedRecord {
            path: "docs/decisions/0002-api.md".to_owned(),
            node: "docs/decisions/README.md".to_owned(),
        }]
    );
}

#[test]
fn a_child_node_its_parent_readme_does_not_list_is_reported() {
    let files = [
        node("docs/decisions/README.md", &[]),
        node("docs/decisions/api/README.md", &[]),
    ];

    assert_eq!(
        check(&files),
        [Violation::UnlistedRecord {
            path: "docs/decisions/api/README.md".to_owned(),
            node: "docs/decisions/README.md".to_owned(),
        }]
    );
}

#[test]
fn a_record_in_a_directory_without_a_readme_is_reported() {
    let files = [
        node("docs/decisions/README.md", &[]),
        record("docs/decisions/api/0001-errors.md"),
    ];

    assert_eq!(
        check(&files),
        [Violation::UnlistedRecord {
            path: "docs/decisions/api/0001-errors.md".to_owned(),
            node: "docs/decisions/api/README.md".to_owned(),
        }]
    );
}

#[test]
fn a_node_link_to_a_missing_file_is_reported() {
    let files = [
        node(
            "docs/decisions/README.md",
            &["0001-errors.md", "0009-gone.md"],
        ),
        record("docs/decisions/0001-errors.md"),
    ];

    assert_eq!(
        check(&files),
        [Violation::BrokenLink {
            path: "docs/decisions/README.md".to_owned(),
            target: "0009-gone.md".to_owned(),
        }]
    );
}

#[test]
fn a_node_link_that_leaves_the_tree_resolves_against_the_node() {
    let files = [
        node(
            "docs/decisions/README.md",
            &["../standards/README.md", "../missing.md"],
        ),
        file("docs/standards/README.md", "# Standards\n"),
    ];

    assert_eq!(
        check(&files),
        [Violation::BrokenLink {
            path: "docs/decisions/README.md".to_owned(),
            target: "../missing.md".to_owned(),
        }]
    );
}

#[test]
fn web_links_and_anchors_in_a_node_are_not_checked_as_files() {
    let files = [file(
        "docs/decisions/README.md",
        "# A node\n\nSee [the guide](https://example.com/guide) and [below](#children).\n",
    )];

    assert_eq!(check(&files), []);
}
