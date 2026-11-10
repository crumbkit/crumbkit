//! Tests for finding and rewriting the generated sections of a skill file.

use super::{Rewrite, Unclosed, rewrite};

/// A skill file with hand-written text around a `jobs` section in HTML
/// comments and a `description` section in YAML comments.
const TEXT: &str = "\
---
name: shelf
# crumbkit:generated description
description: old
# /crumbkit:generated description
---

Hand-written.
<!-- crumbkit:generated jobs -->
old jobs
<!-- /crumbkit:generated jobs -->
More hand-written.
";

#[test]
fn rewriting_a_section_replaces_only_the_lines_between_its_markers() {
    let rewritten = rewrite(TEXT, "jobs", "new jobs\n").unwrap();

    assert_eq!(
        rewritten,
        Rewrite::Changed(TEXT.replace("old jobs\n", "new jobs\n"))
    );
}

#[test]
fn a_section_in_yaml_comments_rewrites_like_one_in_html_comments() {
    let rewritten = rewrite(TEXT, "description", "description: new\n").unwrap();

    assert_eq!(
        rewritten,
        Rewrite::Changed(TEXT.replace("description: old\n", "description: new\n"))
    );
}

#[test]
fn a_section_whose_text_is_already_the_body_is_unchanged() {
    assert_eq!(
        rewrite(TEXT, "jobs", "old jobs\n").unwrap(),
        Rewrite::Unchanged
    );
}

#[test]
fn a_section_without_markers_is_unmarked() {
    assert_eq!(rewrite(TEXT, "usage", "body\n").unwrap(), Rewrite::Unmarked);
}

#[test]
fn every_pair_of_markers_with_the_name_is_rewritten() {
    let twice = format!("{TEXT}{TEXT}");

    let rewritten = rewrite(&twice, "jobs", "new\n").unwrap();

    assert_eq!(
        rewritten,
        Rewrite::Changed(twice.replace("old jobs\n", "new\n"))
    );
}

#[test]
fn a_start_marker_without_an_end_marker_is_unclosed_at_its_line() {
    let text = "a\n<!-- crumbkit:generated jobs -->\nold\n";

    assert_eq!(rewrite(text, "jobs", "new\n"), Err(Unclosed { line: 2 }));
}

#[test]
fn markers_keep_their_own_line_endings_and_indentation() {
    let text = "a\r\n  <!-- crumbkit:generated jobs -->\r\nold\r\n  <!-- /crumbkit:generated jobs -->\r\nb";

    let rewritten = rewrite(text, "jobs", "new\n").unwrap();

    assert_eq!(
        rewritten,
        Rewrite::Changed(
            "a\r\n  <!-- crumbkit:generated jobs -->\r\nnew\n  <!-- /crumbkit:generated jobs -->\r\nb"
                .to_owned()
        )
    );
}
