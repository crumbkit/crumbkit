//! Tests for the text table.

use std::borrow::Cow;

use proptest::prelude::*;
use serde_json::{Value, json};

use super::{FORMAT, Table, cell, escape_controls, is_format};
use crate::Style;

/// Returns the table that `Table::write` prints for `columns` and every
/// row of `rows`, with columns that crumbkit chose, so a column that every
/// row lacks is left out.
fn table(columns: &[&str], rows: &[Value]) -> String {
    written(Table::new(columns, rows, true), rows.len())
}

/// Returns the table that `Table::write` prints for `columns` and every
/// row of `rows`, with columns that `--fields` named, so every column
/// prints.
fn named_table(columns: &[&str], rows: &[Value]) -> String {
    written(Table::new(columns, rows, false), rows.len())
}

/// Returns what `table` writes with its first `rows` rows.
fn written(mut table: Table<'_>, rows: usize) -> String {
    let mut out = Vec::new();
    table.write(&mut out, rows, None, false).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn a_table_written_with_its_first_rows_is_as_wide_as_those_rows_alone() {
    let rows = [
        json!({"id": 1, "title": "Emma"}),
        json!({"id": 2, "title": "Northanger Abbey"}),
    ];
    let mut table = Table::new(&["id", "title"], &rows, true);
    let mut first = Vec::new();
    let mut both = Vec::new();

    table.write(&mut first, 1, None, false).unwrap();
    table.write(&mut both, 2, None, false).unwrap();

    assert_eq!(String::from_utf8(first).unwrap(), "ID  TITLE\n1   Emma\n");
    assert_eq!(
        String::from_utf8(both).unwrap(),
        "ID  TITLE\n1   Emma\n2   Northanger Abbey\n"
    );
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
fn null_and_a_missing_field_print_a_dash_in_a_column_that_another_row_fills() {
    let rows = [
        json!({"id": 1, "year": null}),
        json!({"id": 2, "year": 1818, "pages": 280}),
    ];

    let expected = "\
ID  YEAR  PAGES
1   -     -
2   1818  280
";
    assert_eq!(table(&["id", "year", "pages"], &rows), expected);
}

#[test]
fn a_column_that_is_null_in_every_row_is_left_out_of_the_header_and_the_rows() {
    let rows = [
        json!({"id": 1, "state": null, "title": "Emma"}),
        json!({"id": 2, "state": null, "title": "Persuasion"}),
    ];

    let expected = "\
ID  TITLE
1   Emma
2   Persuasion
";
    assert_eq!(table(&["id", "state", "title"], &rows), expected);
}

#[test]
fn a_column_that_every_row_is_missing_is_left_out() {
    let rows = [
        json!({"id": 1, "title": "Emma"}),
        json!({"id": 2, "title": "Persuasion"}),
    ];

    assert_eq!(
        table(&["id", "title", "state"], &rows),
        "ID  TITLE\n1   Emma\n2   Persuasion\n"
    );
}

#[test]
fn a_last_column_left_out_leaves_no_padding_at_the_end_of_a_line() {
    let rows = [json!({"title": "Emma", "id": 1, "state": null})];

    assert_eq!(
        table(&["title", "id", "state"], &rows),
        "TITLE  ID\nEmma   1\n"
    );
}

#[test]
fn a_column_that_is_null_in_only_some_rows_keeps_its_column() {
    let rows = [
        json!({"id": 1, "state": null}),
        json!({"id": 2, "state": "lent"}),
    ];

    let expected = "\
ID  STATE
1   -
2   lent
";
    assert_eq!(table(&["id", "state"], &rows), expected);
}

#[test]
fn a_string_cell_that_holds_a_dash_keeps_its_column() {
    let rows = [
        json!({"id": 1, "state": "-"}),
        json!({"id": 2, "state": null}),
    ];

    let expected = "\
ID  STATE
1   -
2   -
";
    assert_eq!(table(&["id", "state"], &rows), expected);
}

#[test]
fn a_named_column_that_is_null_in_every_row_still_prints() {
    let rows = [json!({"id": 1, "state": null})];

    assert_eq!(named_table(&["id", "state"], &rows), "ID  STATE\n1   -\n");
}

#[test]
fn a_column_filled_only_in_a_row_left_unwritten_is_left_out() {
    let rows = [
        json!({"id": 1, "state": null}),
        json!({"id": 2, "state": "lent"}),
    ];
    let table = Table::new(&["id", "state"], &rows, true);

    assert_eq!(written(table, 1), "ID\n1\n");
}

#[test]
fn a_table_written_with_more_rows_again_brings_back_a_column_that_they_fill() {
    let rows = [
        json!({"id": 1, "state": null}),
        json!({"id": 2, "state": "lent"}),
    ];
    let mut table = Table::new(&["id", "state"], &rows, true);
    let mut first = Vec::new();
    let mut both = Vec::new();

    table.write(&mut first, 1, None, false).unwrap();
    table.write(&mut both, 2, None, false).unwrap();

    assert_eq!(String::from_utf8(first).unwrap(), "ID\n1\n");
    assert_eq!(
        String::from_utf8(both).unwrap(),
        "ID  STATE\n1   -\n2   lent\n"
    );
}

#[test]
fn a_table_without_rows_prints_every_column_in_its_header() {
    assert_eq!(table(&["id", "state"], &[]), "ID  STATE\n");
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
fn a_row_of_empty_cells_prints_as_an_empty_line() {
    let rows = [json!({"id": "", "title": "", "note": ""})];

    assert_eq!(
        table(&["id", "title", "note"], &rows),
        "ID  TITLE  NOTE\n\n"
    );
}

#[test]
fn an_empty_middle_cell_is_still_padded_when_a_later_cell_has_text() {
    let rows = [json!({"id": "1", "title": "", "note": "x"})];

    let expected = "\
ID  TITLE  NOTE
1          x
";
    assert_eq!(table(&["id", "title", "note"], &rows), expected);
}

#[test]
fn a_last_cell_that_ends_in_a_space_keeps_its_space() {
    let rows = [json!({"id": "1", "note": "x "})];

    assert_eq!(table(&["id", "note"], &rows), "ID  NOTE\n1   x \n");
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
fn a_bidirectional_override_in_a_cell_prints_as_its_unicode_escape() {
    let rows = [json!({"note": "abc\u{202e}def"})];

    assert_eq!(table(&["note"], &rows), "NOTE\nabc\\u{202e}def\n");
}

#[test]
fn a_soft_hyphen_prints_as_its_unicode_escape() {
    assert_eq!(escape_controls("co\u{ad}operate"), "co\\u{ad}operate");
}

#[test]
fn ordinary_text_with_letters_outside_ascii_is_borrowed_unchanged() {
    let text = "Brontë é 日本 \u{301}";

    let escaped = escape_controls(text);

    assert!(
        matches!(escaped, Cow::Borrowed(borrowed) if borrowed == text),
        "{escaped:?}"
    );
}

#[test]
fn the_format_character_table_is_sorted_and_its_ranges_do_not_overlap() {
    for (&(first, last), &(next, _)) in FORMAT.iter().zip(FORMAT.iter().skip(1)) {
        assert!(first <= last && last < next, "{first:x} {last:x} {next:x}");
    }
}

#[test]
fn the_first_and_last_code_point_of_each_format_range_is_a_format_character() {
    for &(first, last) in FORMAT {
        for point in [first, last] {
            let character = char::from_u32(point).unwrap();
            assert!(is_format(character), "{point:x}");
        }
    }
}

#[test]
fn a_code_point_next_to_a_format_range_is_not_a_format_character() {
    for &(first, last) in FORMAT {
        for point in [first - 1, last + 1] {
            let character = char::from_u32(point).unwrap();
            assert!(!is_format(character), "{point:x}");
        }
    }
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

/// The columns of the generated tables, which give a first, a middle and a
/// last column.
const COLUMNS: [&str; 3] = ["id", "title", "note"];

/// Generates the text of one cell: up to six characters drawn from lowercase
/// letters, a space, every C0 and C1 control character and DEL, `é` (two bytes
/// in UTF-8), `界` (a CJK character that a terminal shows two columns wide),
/// `🦀` (four bytes in UTF-8), U+0301, a combining accent, and the format
/// characters U+200B, a zero-width space, and U+202E, a right-to-left
/// override. The empty string is one of the texts.
const CELL_TEXT: &str = r"[a-z \x00-\x1f\x7f-\x9fé界🦀\u{301}\u{200b}\u{202e}]{0,6}";

/// Generates up to five rows of three cells.
fn rows() -> impl Strategy<Value = Vec<[String; 3]>> {
    prop::collection::vec(prop::array::uniform3(CELL_TEXT), 0..6)
}

/// Returns each row as the JSON object that `write` reads, with a field for
/// each of `COLUMNS`.
fn values(rows: &[[String; 3]]) -> Vec<Value> {
    rows.iter()
        .map(|[id, title, note]| json!({"id": id, "title": title, "note": note}))
        .collect()
}

proptest! {
    /// `write` promises that each column is as wide as its widest cell, that
    /// two spaces separate the columns and that a line ends with its last
    /// non-empty cell. So each column starts at the same character offset on
    /// every line: the first at 0, and each other column two characters after
    /// the widest cell of the column before it. The test builds each expected
    /// line by placing every non-empty cell at its column's offset.
    ///
    /// The offsets count characters, as `write` does. A terminal shows `界`
    /// and `🦀` two columns wide and U+0301 zero wide, so a table holding them
    /// misaligns on screen, which `write` documents.
    #[test]
    fn a_table_has_one_line_per_row_without_raw_control_or_format_characters_and_aligned_columns(
        rows in rows(),
    ) {
        let printed = table(&COLUMNS, &values(&rows));

        // The cells as the table shows them, header first: the column names
        // in upper case, and each string with its control and format
        // characters escaped.
        let mut lines = vec![COLUMNS.map(str::to_uppercase)];
        for row in &rows {
            lines.push(row.clone().map(|text| escape_controls(&text).into_owned()));
        }
        let mut starts = [0; 3];
        for column in 1..3 {
            let widest = lines
                .iter()
                .map(|line| line[column - 1].chars().count())
                .max();
            starts[column] = starts[column - 1] + widest.unwrap_or(0) + 2;
        }

        let printed_lines: Vec<&str> = printed.split_terminator('\n').collect();
        prop_assert!(printed.ends_with('\n'));
        prop_assert_eq!(printed_lines.len(), rows.len() + 1);
        for (printed_line, cells) in printed_lines.iter().zip(&lines) {
            prop_assert!(
                !printed_line
                    .chars()
                    .any(|character| character.is_control() || is_format(character)),
                "{printed_line:?}"
            );

            let mut expected = String::new();
            for (cell, start) in cells.iter().zip(starts) {
                if cell.is_empty() {
                    continue;
                }
                let padding = start - expected.chars().count();
                expected.push_str(&" ".repeat(padding));
                expected.push_str(cell);
            }
            prop_assert_eq!(*printed_line, expected);
        }
    }
}

#[test]
fn a_table_with_styles_writes_them_only_when_asked_and_pads_after_each_reset() {
    let rows = [
        json!({"name": "web", "state": "running"}),
        json!({"name": "queue-worker", "state": "failed"}),
    ];
    let cell_styles = vec![
        None,
        Some(Style::Success),
        Some(Style::Emphasis),
        Some(Style::Error),
    ];
    let mut table = Table::new(&["name", "state"], &rows, true).with_styles(cell_styles);
    let mut plain = Vec::new();
    let mut styled = Vec::new();
    table.write(&mut plain, 2, None, false).unwrap();
    table.write(&mut styled, 2, None, true).unwrap();

    let expected_plain = "\
NAME          STATE
web           running
queue-worker  failed
";
    let expected_styled = "\
NAME          STATE
web           \u{1b}[32mrunning\u{1b}[0m
\u{1b}[1mqueue-worker\u{1b}[0m  \u{1b}[31mfailed\u{1b}[0m
";
    assert_eq!(String::from_utf8(plain).unwrap(), expected_plain);
    assert_eq!(String::from_utf8(styled).unwrap(), expected_styled);
}
