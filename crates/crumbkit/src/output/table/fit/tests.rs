//! Tests for fitting the text table to a terminal's width (ADR-0121).

use std::borrow::Cow;

use serde_json::{Value, json};

use super::{cut, fit};
use crate::output::table::Table;

/// Returns the header line of a table with `names` as its headers.
fn header(names: &[&'static str]) -> Vec<Cow<'static, str>> {
    names.iter().copied().map(Cow::Borrowed).collect()
}

/// Returns the table that `Table::write` prints for `columns` and every
/// row of `rows` at a terminal `width` columns wide, with the columns in
/// `uncut` kept whole.
fn table(columns: &[&str], uncut: &[&str], rows: &[Value], width: Option<usize>) -> String {
    let mut out = Vec::new();
    Table::new(columns, rows)
        .keeping_whole(columns, uncut)
        .write(&mut out, rows.len(), width)
        .unwrap();
    String::from_utf8(out).unwrap()
}

/// Returns three books whose titles are the widest cells of their rows.
fn books() -> [Value; 3] {
    [
        json!({"id": 1, "title": "Pride and Prejudice", "author": "Jane Austen"}),
        json!({"id": 2, "title": "Frankenstein", "author": "Mary Shelley"}),
        json!({"id": 6, "title": "The Adventures of Sherlock Holmes", "author": "Arthur Conan Doyle"}),
    ]
}

#[test]
fn a_table_narrower_than_the_terminal_prints_unchanged() {
    let rows = books();

    let fitted = table(&["id", "title", "author"], &[], &rows, Some(200));

    assert_eq!(fitted, table(&["id", "title", "author"], &[], &rows, None));
}

#[test]
fn the_widest_column_narrows_first_and_a_cut_cell_ends_with_an_ellipsis() {
    let rows = books();

    let fitted = table(&["id", "title", "author"], &[], &rows, Some(41));

    let expected = "\
ID  TITLE              AUTHOR
1   Pride and Prejud…  Jane Austen
2   Frankenstein       Mary Shelley
6   The Adventures o…  Arthur Conan Doyle
";
    assert_eq!(fitted, expected);
    assert!(
        fitted.lines().all(|line| line.chars().count() <= 41),
        "a line is wider than 41 characters:\n{fitted}"
    );
}

#[test]
fn a_table_exactly_as_wide_as_the_terminal_cuts_nothing() {
    let rows = books();
    let unfitted = table(&["id", "title", "author"], &[], &rows, None);
    let width = unfitted
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap();

    let fitted = table(&["id", "title", "author"], &[], &rows, Some(width));

    assert_eq!(fitted, unfitted);
}

#[test]
fn a_column_never_narrows_below_its_header() {
    let mut widths = [2, 33, 10];

    let narrowed = fit(&mut widths, &header(&["ID", "TITLE", "AUTHOR"]), &[], 20);

    // The excess of 29 would take the title to 4 characters, one fewer
    // than its header, so the title stops at 5 and the author narrows by
    // the 1 left.
    assert!(narrowed, "the table was not narrowed");
    assert_eq!(widths, [2, 5, 9]);
}

#[test]
fn the_next_widest_column_narrows_when_the_first_reaches_its_header() {
    let mut widths = [2, 33, 18];

    fit(&mut widths, &header(&["ID", "TITLE", "AUTHOR"]), &[], 25);

    // A line is 2 + 33 + 18 characters and two separators of 2, which is
    // 57, so the excess is 32. The title narrows to its header's 5
    // characters, which leaves an excess of 4 for the author.
    assert_eq!(widths, [2, 5, 14]);
}

#[test]
fn of_two_columns_equally_wide_the_leftmost_narrows_first() {
    let mut widths = [10, 10];

    fit(&mut widths, &header(&["A", "B"]), &[], 18);

    assert_eq!(widths, [6, 10]);
}

#[test]
fn a_table_whose_headers_alone_are_wider_than_the_terminal_prints_unchanged() {
    let rows = books();
    let unfitted = table(&["id", "title", "author"], &[], &rows, None);

    let fitted = table(&["id", "title", "author"], &[], &rows, Some(12));

    assert_eq!(fitted, unfitted);
    let mut widths = [2, 33, 18];
    assert!(
        !fit(&mut widths, &header(&["ID", "TITLE", "AUTHOR"]), &[], 12),
        "the fit narrowed a table whose headers alone do not fit"
    );
    assert_eq!(widths, [2, 33, 18]);
}

#[test]
fn an_uncut_column_keeps_its_width_while_the_next_widest_narrows() {
    let rows = books();

    let fitted = table(&["id", "title", "author"], &["title"], &rows, Some(50));

    let expected = "\
ID  TITLE                              AUTHOR
1   Pride and Prejudice                Jane Austen
2   Frankenstein                       Mary Shell…
6   The Adventures of Sherlock Holmes  Arthur Con…
";
    assert_eq!(fitted, expected);
}

#[test]
fn a_table_that_cannot_fit_with_its_uncut_columns_whole_prints_overflowing() {
    let rows = books();
    let unfitted = table(&["id", "title", "author"], &[], &rows, None);

    let fitted = table(&["id", "title", "author"], &["title"], &rows, Some(40));

    assert_eq!(fitted, unfitted);
    assert!(
        fitted.lines().any(|line| line.chars().count() > 40),
        "no line overflows the terminal:\n{fitted}"
    );
}

#[test]
fn a_cut_cell_keeps_its_first_characters_and_ends_with_one_ellipsis() {
    assert_eq!(cut("Frankenstein", 6), "Frank…");
    assert_eq!(cut("Frankenstein", 12), "Frankenstein");
    assert_eq!(cut("Émile", 3), "Ém…");
}
