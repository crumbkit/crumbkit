//! Tests for the `prints_no_result` field, which marks the row of a command
//! whose leaf declares that it prints no result (ADR-0169).

use serde_json::Value;

use super::{args, keys, output, required, row};
use crate::commands::{CommandRow, list};
use crate::help::{Guide, Leaf};

/// A guide that declares that `books show` prints no result and gives
/// `books list` a leaf without a document, which offers every view flag.
const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "show"]).prints_no_result(),
    Leaf::new(&["books", "list"]),
]);

/// Lists the `books` branch with [`GUIDE`] and returns the rows as JSON.
fn rows() -> Value {
    serde_json::to_value(list(&mut output(), &GUIDE, &args(&["books"])).unwrap()).unwrap()
}

#[test]
fn the_row_of_a_leaf_that_prints_no_result_has_prints_no_result_true_and_no_form() {
    let rows = rows();
    let show = row(&rows, "shelf books show");

    assert_eq!(show["prints_no_result"], Value::Bool(true));
    assert_eq!(show["form"], Value::Null);
}

#[test]
fn the_row_of_a_leaf_that_prints_no_result_has_the_schema_s_required_fields_and_prints_no_result() {
    let rows = rows();
    let schema = schemars::schema_for!(CommandRow);
    let schema = schema.as_value();
    let mut expected = required(schema);
    expected.push("prints_no_result");
    expected.sort_unstable();

    assert_eq!(keys(row(&rows, "shelf books show")), expected);
    assert!(schema["properties"]["prints_no_result"].is_object());
}

#[test]
fn the_row_of_a_leaf_without_a_document_has_no_prints_no_result_field_and_no_form() {
    let rows = rows();
    let list = row(&rows, "shelf books list");

    assert!(list.get("prints_no_result").is_none());
    assert_eq!(list["form"], Value::Null);
}

#[test]
fn the_row_of_a_command_without_a_leaf_has_no_prints_no_result_field() {
    let rows = rows();

    assert!(row(&rows, "shelf books").get("prints_no_result").is_none());
}
