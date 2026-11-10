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
fn the_body_takes_the_line_ending_of_its_start_marker() {
    let text = "a\r\n  <!-- crumbkit:generated jobs -->\r\nold\r\n  <!-- /crumbkit:generated jobs -->\r\nb";

    let rewritten = rewrite(text, "jobs", "new\nlines\n").unwrap();

    assert_eq!(
        rewritten,
        Rewrite::Changed(
            "a\r\n  <!-- crumbkit:generated jobs -->\r\nnew\r\nlines\r\n  <!-- /crumbkit:generated jobs -->\r\nb"
                .to_owned()
        )
    );
    let unchanged = text.replace("old", "new\r\nlines");
    assert_eq!(
        rewrite(&unchanged, "jobs", "new\nlines\n").unwrap(),
        Rewrite::Unchanged
    );
}

#[test]
fn markers_inside_a_fenced_code_block_are_example_text() {
    for fence in ["```", "~~~"] {
        let text = format!(
            "{fence}markdown\n<!-- crumbkit:generated jobs -->\nexample\n<!-- /crumbkit:generated jobs -->\n{fence}\n"
        );

        assert_eq!(rewrite(&text, "jobs", "new\n").unwrap(), Rewrite::Unmarked);
    }
}

#[test]
fn a_fence_of_the_other_kind_does_not_close_a_fenced_block() {
    let text = "```\n~~~\n<!-- crumbkit:generated jobs -->\n```\n";

    assert_eq!(rewrite(text, "jobs", "new\n").unwrap(), Rewrite::Unmarked);
}

#[test]
fn yaml_markers_count_only_in_frontmatter_and_html_markers_only_after_it() {
    let body = "# crumbkit:generated description\nheading\n# /crumbkit:generated description\n";
    let front =
        "---\n<!-- crumbkit:generated jobs -->\nx\n<!-- /crumbkit:generated jobs -->\n---\n";

    assert_eq!(
        rewrite(body, "description", "new\n").unwrap(),
        Rewrite::Unmarked
    );
    assert_eq!(rewrite(front, "jobs", "new\n").unwrap(), Rewrite::Unmarked);
}
