//! Tests for the `CommonMark` text helpers.

use super::{anchor, code, fence, paragraphs, sentence};

#[test]
fn inline_markup_characters_are_escaped_so_they_print_as_written() {
    assert_eq!(
        paragraphs("Print *all* rows of [a] <b> & c_d ~e\\f"),
        ["Print \\*all\\* rows of \\[a\\] \\<b\\> \\& c\\_d \\~e\\\\f"]
    );
}

#[test]
fn a_code_span_in_the_text_is_kept_without_escapes() {
    assert_eq!(
        paragraphs("Run `shelf --fields <FIELD>` or <N>"),
        ["Run `shelf --fields <FIELD>` or \\<N\\>"]
    );
    assert_eq!(
        paragraphs("A ``tick ` inside`` and `open"),
        ["A ``tick ` inside`` and `open"]
    );
}

#[test]
fn a_marker_at_the_start_of_a_line_is_escaped_so_no_line_starts_a_block() {
    assert_eq!(
        paragraphs("# one\n- two\n+ three\n1. four\n12) five\n= six\n```seven"),
        ["\\# one\n\\- two\n\\+ three\n1\\. four\n12\\) five\n\\= six\n\\```seven"]
    );
}

#[test]
fn indentation_is_removed_so_no_line_starts_a_code_block() {
    assert_eq!(paragraphs("    indented\n  two"), ["indented\ntwo"]);
}

#[test]
fn a_blank_line_separates_paragraphs_and_empty_text_has_none() {
    assert_eq!(
        paragraphs("First line\nsecond line\n\n \nNext paragraph\n"),
        ["First line\nsecond line", "Next paragraph"]
    );
    assert_eq!(paragraphs(" \n"), Vec::<String>::new());
}

#[test]
fn a_code_span_uses_more_backticks_than_any_run_inside_it() {
    assert_eq!(code("--limit <N>"), "`--limit <N>`");
    assert_eq!(code("a`b"), "``a`b``");
    assert_eq!(code("`a"), "`` `a ``");
}

#[test]
fn a_fence_is_longer_than_any_run_of_backticks_in_the_block() {
    assert_eq!(fence("shelf books list"), "```");
    assert_eq!(fence("echo ````"), "`````");
}

#[test]
fn an_anchor_is_the_heading_lowercased_with_spaces_as_dashes_and_other_punctuation_dropped() {
    assert_eq!(anchor("shelf books list"), "shelf-books-list");
    assert_eq!(anchor("Commands of shelf"), "commands-of-shelf");
    assert_eq!(anchor("tool self_check v1.2"), "tool-self_check-v12");
}

#[test]
fn a_sentence_ends_with_a_full_stop_unless_it_already_ends_with_punctuation() {
    assert_eq!(sentence("Print every row"), "Print every row.");
    assert_eq!(sentence("Print every row."), "Print every row.");
    assert_eq!(sentence("Really?"), "Really?");
    assert_eq!(sentence("Run `shelf`"), "Run `shelf`.");
}
