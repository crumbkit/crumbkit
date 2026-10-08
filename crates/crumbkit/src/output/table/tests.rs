//! Tests for the text table.

use std::borrow::Cow;

use serde_json::{Value, json};

use super::{cell, write};

/// Returns the table that `write` prints for `columns` and `rows`.
fn table(columns: &[&str], rows: &[Value]) -> String {
    let mut out = Vec::new();
    write(&mut out, columns, rows).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn each_column_is_as_wide_as_its_widest_cell_and_the_last_column_is_not_padded() {
    let rows = [
        json!({"id": 1, "title": "Emma", "year": 1815}),
        json!({"id": 10, "title": "Persuasion", "year": 1817}),
    ];

    let expected = "\
ID  TITLE       YEAR
1   Emma        1815
10  Persuasion  1817
";
    assert_eq!(table(&["id", "title", "year"], &rows), expected);
}

#[test]
fn a_header_wider_than_every_cell_sets_the_column_width_and_no_line_ends_in_spaces() {
    let rows = [json!({"n": 1, "author": "Li"})];

    let expected = "\
N  AUTHOR
1  Li
";
    assert_eq!(table(&["n", "author"], &rows), expected);
}

#[test]
fn an_empty_result_prints_the_header_only() {
    assert_eq!(table(&["id", "title"], &[]), "ID  TITLE\n");
}

#[test]
fn a_string_prints_as_it_is_without_quotes() {
    let rows = [json!({"title": "Moby-Dick", "note": "\"quoted\""})];

    let expected = "\
TITLE      NOTE
Moby-Dick  \"quoted\"
";
    assert_eq!(table(&["title", "note"], &rows), expected);
}

#[test]
fn numbers_and_booleans_print_in_their_json_form() {
    let rows = [json!({"count": -3, "ratio": 0.5, "done": true})];

    let expected = "\
COUNT  RATIO  DONE
-3     0.5    true
";
    assert_eq!(table(&["count", "ratio", "done"], &rows), expected);
}

#[test]
fn null_and_a_missing_field_print_a_dash() {
    let rows = [json!({"id": 1, "year": null})];

    let expected = "\
ID  YEAR  PAGES
1   -     -
";
    assert_eq!(table(&["id", "year", "pages"], &rows), expected);
}

#[test]
fn arrays_and_objects_print_as_compact_json() {
    let rows = [json!({"tags": ["gothic", "epistolary"], "meta": {"pages": 280}})];

    let expected = "\
TAGS                     META
[\"gothic\",\"epistolary\"]  {\"pages\":280}
";
    assert_eq!(table(&["tags", "meta"], &rows), expected);
}

#[test]
fn width_counts_characters_so_text_outside_ascii_still_aligns() {
    let rows = [
        json!({"author": "Brontë", "year": 1847}),
        json!({"author": "Eliot", "year": 1871}),
    ];

    let expected = "\
AUTHOR  YEAR
Brontë  1847
Eliot   1871
";
    assert_eq!(table(&["author", "year"], &rows), expected);
}

#[test]
fn a_newline_in_a_string_prints_as_backslash_n_so_the_row_stays_on_one_line() {
    let rows = [json!({"id": 1, "note": "first\nsecond"})];

    let expected = "\
ID  NOTE
1   first\\nsecond
";
    assert_eq!(table(&["id", "note"], &rows), expected);
}

#[test]
fn a_tab_in_a_string_prints_as_backslash_t_and_the_width_counts_the_escape() {
    let rows = [
        json!({"note": "a\tb", "id": 1}),
        json!({"note": "c", "id": 2}),
    ];

    let expected = "\
NOTE  ID
a\\tb  1
c     2
";
    assert_eq!(table(&["note", "id"], &rows), expected);
}

#[test]
fn another_control_character_prints_as_its_unicode_escape() {
    let rows = [json!({"note": "\u{1b}[1mbold"})];

    assert_eq!(table(&["note"], &rows), "NOTE\n\\u{1b}[1mbold\n");
}

#[test]
fn a_string_without_a_control_character_is_borrowed_unchanged() {
    let value = json!("Brontë \"quoted\" \\ text");

    let cell = cell(Some(&value));

    assert!(
        matches!(cell, Cow::Borrowed("Brontë \"quoted\" \\ text")),
        "{cell:?}"
    );
}
