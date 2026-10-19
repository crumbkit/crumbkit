//! Rows that report a problem, as `Row::is_problem` decides, by default
//! through the field that `Row::PROBLEM` names (ADR-0050, ADR-0096).
//! crumbkit reads these functions to choose the exit code and word the
//! note, and they are not part of the public API.

use serde_json::Value;

use crate::Row;

/// Returns the number of `rows` that report a problem, as
/// [`Row::is_problem`] decides. A row type that keeps the default and names
/// no problem field reports none, and its rows are not serialized.
#[must_use]
pub fn count<R: Row>(rows: &[R]) -> usize {
    rows.iter().filter(|row| row.is_problem()).count()
}

/// Reports whether `row` reports a problem through the field that
/// `R::PROBLEM` names: whether that field serializes to a value other than
/// `null`, `false` or an empty string. It is the default of
/// [`Row::is_problem`], and a row type that names no field reports none.
///
/// A row that cannot be serialized reports no problem here, because the
/// write of the result reports the error and exits with 1.
#[must_use]
pub fn field_reports<R: Row + ?Sized>(row: &R) -> bool {
    let Some(field) = R::PROBLEM else {
        return false;
    };
    let Ok(value) = serde_json::to_value(row) else {
        return false;
    };
    match value.get(field) {
        None | Some(Value::Null | Value::Bool(false)) => false,
        Some(Value::String(text)) => !text.is_empty(),
        Some(_) => true,
    }
}
