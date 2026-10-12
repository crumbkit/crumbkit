//! Tests for how a failure prints the labels of its miette diagnostic as
//! text.

use miette::{LabeledSpan, NamedSource};

use super::super::super::{PreparedFailure, write_crumbkit_text};
use super::super::tests::{SHELF_TOML, UnknownKey, in_shelf_toml, on_colour};
use crate::{Failure, Spelling};

/// A source of 12 lines whose array opens on line 1 and whose last value is
/// on line 11. The `[` is at byte 4, and line `n` from 2 to 11 starts at byte
/// `6 + 5 * (n - 2)`.
const LONG_ARRAY: &str =
    "x = [\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\ny = 2\n";

/// Returns the label from the `[` on line 1 of `LONG_ARRAY` to the end of
/// line 11.
fn over_long_array() -> LabeledSpan {
    LabeledSpan::new(Some("this array is never closed".to_owned()), 4, 51)
}

/// Returns the text form of `failure`.
fn text(failure: &dyn Failure) -> String {
    let document = PreparedFailure::new(failure, Spelling::COLONS, None, &mut std::io::sink());
    let mut err = Vec::new();
    write_crumbkit_text(&mut err, &document).unwrap();
    String::from_utf8(err).unwrap()
}

#[test]
fn a_label_prints_an_at_line_then_its_source_line_with_a_caret_underline_between_the_lines_around_it()
 {
    let failure = in_shelf_toml(vec![on_colour()]);

    let expected = "\
error: unknown key `colour`
  at:    shelf.toml:3:1: this key is not known
     2 | name = \"mine\"
     3 | colour = \"red\"
       | ^^^^^^
     4 | size = 3
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_label_on_a_one_line_unnamed_source_prints_its_line_and_column_and_no_lines_around_it() {
    let failure = UnknownKey {
        source: "colour = \"red\"\n",
        labels: vec![LabeledSpan::new(Some("not a colour".to_owned()), 9, 5)],
    };

    let expected = "\
error: unknown key `colour`
  at:    1:10: not a colour
     1 | colour = \"red\"
       |          ^^^^^
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_label_without_text_prints_only_its_place() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 22, 6)]);

    assert!(
        text(&failure).contains("\n  at:    shelf.toml:3:1\n     2 | name"),
        "{}",
        text(&failure)
    );
}

#[test]
fn a_span_over_two_lines_is_underlined_from_its_start_on_the_first_and_up_to_its_end_on_the_last() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 31, 10)]);

    let expected = "  at:    shelf.toml:3:10
     2 | name = \"mine\"
     3 | colour = \"red\"
       |          ^^^^^
     4 | size = 3
       | ^^^^
  help:";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_span_that_ends_with_its_line_break_covers_only_that_line() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 22, 15)]);

    let expected = "     3 | colour = \"red\"
       | ^^^^^^^^^^^^^^
     4 | size = 3
  help:";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_span_over_seven_lines_prints_every_line() {
    let failure = UnknownKey {
        source: "x = [\n  1,\n  1,\n  1,\n  1,\n  1,\n  1,\ny = 2\n",
        labels: vec![LabeledSpan::new(None, 4, 31)],
    };

    let expected = "     1 | x = [
       |     ^
     2 |   1,
     3 |   1,
     4 |   1,
     5 |   1,
     6 |   1,
     7 |   1,
       | ^^^^
     8 | y = 2
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_span_over_more_than_seven_lines_prints_its_first_and_last_three_and_counts_the_lines_left_out()
{
    let failure = UnknownKey {
        source: LONG_ARRAY,
        labels: vec![over_long_array()],
    };

    let expected = "\
error: unknown key `colour`
  at:    1:5: this array is never closed
     1 | x = [
       |     ^
     2 |   1,
     3 |   1,
   ... | 5 lines left out
     9 |   1,
    10 |   1,
    11 |   1,
       | ^^^^
    12 | y = 2
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn two_labels_whose_lines_touch_print_their_at_lines_in_order_then_each_line_once_with_each_text_after_its_carets()
 {
    let failure = in_shelf_toml(vec![
        on_colour(),
        LabeledSpan::new(Some("the section starts here".to_owned()), 0, 7),
    ]);

    let expected = "\
error: unknown key `colour`
  at:    shelf.toml:3:1: this key is not known
  at:    shelf.toml:1:1: the section starts here
     1 | [shelf]
       | ^^^^^^^ the section starts here
     2 | name = \"mine\"
     3 | colour = \"red\"
       | ^^^^^^ this key is not known
     4 | size = 3
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn two_labels_on_one_line_underline_it_in_two_rows_in_the_order_the_diagnostic_gives_them() {
    let failure = in_shelf_toml(vec![
        on_colour(),
        LabeledSpan::new(Some("this value is ignored".to_owned()), 31, 5),
    ]);

    let expected = "     3 | colour = \"red\"
       | ^^^^^^ this key is not known
       |          ^^^^^ this value is ignored
     4 | size = 3
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn two_labels_with_a_line_between_their_lines_print_separate_blocks() {
    let failure = UnknownKey {
        source: NamedSource::new("shelf.toml", format!("{SHELF_TOML}[loans]\ndays = 14\n")),
        labels: vec![
            LabeledSpan::new(Some("the section starts here".to_owned()), 0, 7),
            LabeledSpan::new(Some("another section starts here".to_owned()), 46, 7),
        ],
    };

    let expected = "\
  at:    shelf.toml:1:1: the section starts here
     1 | [shelf]
       | ^^^^^^^
     2 | name = \"mine\"
  at:    shelf.toml:5:1: another section starts here
     4 | size = 3
     5 | [loans]
       | ^^^^^^^
     6 | days = 14
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_label_inside_a_long_span_prints_the_lines_between_them_when_only_one_line_would_be_left_out() {
    let failure = UnknownKey {
        source: LONG_ARRAY,
        labels: vec![
            over_long_array(),
            LabeledSpan::new(Some("this value repeats".to_owned()), 28, 1),
        ],
    };

    let expected = "\
  at:    1:5: this array is never closed
  at:    6:3: this value repeats
     1 | x = [
       |     ^ this array is never closed
     2 |   1,
     3 |   1,
     4 |   1,
     5 |   1,
     6 |   1,
       |   ^ this value repeats
     7 |   1,
     8 |   1,
     9 |   1,
    10 |   1,
    11 |   1,
       | ^^^^ this array is never closed
    12 | y = 2
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn the_carets_after_wide_characters_start_under_the_labelled_text() {
    // Each of the three CJK characters takes two terminal columns, so the
    // 24 characters before `colour` take 27 columns. The `at:` column
    // counts characters, as an editor does.
    let failure = UnknownKey {
        source: "[shelf]\nbooks = { name = \"猫の本\", colour = \"red\" }\nsize = 3\n",
        labels: vec![LabeledSpan::new(None, 38, 6)],
    };

    let expected = "  at:    2:25
     1 | [shelf]
     2 | books = { name = \"猫の本\", colour = \"red\" }
       |                            ^^^^^^
     3 | size = 3
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn an_empty_span_is_underlined_with_one_caret() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 28, 0)]);

    let expected = "     3 | colour = \"red\"
       |       ^
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn an_empty_source_line_prints_without_a_space_after_its_bar() {
    let failure = UnknownKey {
        source: "a = 1\n\nb = x\n",
        labels: vec![LabeledSpan::new(None, 11, 1)],
    };

    let expected = "  at:    3:5
     2 |
     3 | b = x
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_span_that_cannot_be_read_prints_its_byte_offset_and_no_source_line() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(
        Some("this key is not known".to_owned()),
        400,
        6,
    )]);

    let expected = "\
error: unknown key `colour`
  at:    byte 400: this key is not known
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_control_character_in_the_source_line_or_the_label_prints_as_its_escape() {
    let failure = UnknownKey {
        source: "\tcolour = \"red\"\r\n",
        labels: vec![LabeledSpan::new(Some("not\nknown".to_owned()), 1, 6)],
    };

    let expected = "  at:    1:2: not\\nknown
     1 | \\tcolour = \"red\"
       |   ^^^^^^
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}
