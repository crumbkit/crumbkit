//! Tests for finding the code in a Markdown file.

use super::{Code, code};

/// Returns the line number and text of each piece of code in `text`.
fn found(text: &str) -> Vec<(usize, String)> {
    code(text)
        .into_iter()
        .map(|Code { line, text }| (line, text))
        .collect()
}

#[test]
fn each_inline_code_span_is_found_with_its_line_number() {
    let text = "# Shelf\n\nRun `shelf books list` or `shelf books show ID`.\n";

    assert_eq!(
        found(text),
        [
            (3, "shelf books list".to_owned()),
            (3, "shelf books show ID".to_owned())
        ]
    );
}

#[test]
fn a_span_of_double_backticks_holds_single_ones_and_loses_one_padding_space() {
    assert_eq!(
        found("Type `` shelf books show `id` `` here.\n"),
        [(1, "shelf books show `id`".to_owned())]
    );
}

#[test]
fn an_unmatched_backtick_is_text() {
    assert_eq!(
        found("Use `shelf`, and a ` alone.\n"),
        [(1, "shelf".to_owned())]
    );
    assert_eq!(found("A `` pair never closed.\n"), []);
}

#[test]
fn each_line_of_a_fenced_block_is_found_and_its_spans_are_not() {
    let text = "Intro\n\n```sh\nshelf books list\n\n  shelf books show `x`\n```\n\n~~~~\nshelf schemas list\n~~~\n~~~~\nAfter `shelf`.\n";

    assert_eq!(
        found(text),
        [
            (4, "shelf books list".to_owned()),
            (6, "shelf books show `x`".to_owned()),
            (10, "shelf schemas list".to_owned()),
            (11, "~~~".to_owned()),
            (13, "shelf".to_owned()),
        ]
    );
}

#[test]
fn a_block_that_is_never_closed_runs_to_the_end_of_the_file() {
    assert_eq!(
        found("```\nshelf books list\n"),
        [(2, "shelf books list".to_owned())]
    );
}
