//! The default child lines of a row: the text of the items of its children
//! field.

use serde::Serialize;
use serde_json::Value;

use crate::shown::TextCells;

#[cfg(test)]
mod tests;

/// Returns the items of the field `children` of `row`, serialized as text
/// cells are, when the field holds an array whose every item is a string,
/// and no lines otherwise. A row without a children field, or one that does
/// not serialize, has no lines.
//
// RUST-10: this serializes the whole row a second time, into a
// `serde_json::Value`, for each row whose type sets `Row::CHILDREN` and keeps
// the default `child_lines`, although the table, `Room` and a stream's text
// line have already serialized that row in the text mode. A row type without
// `CHILDREN` never reaches it, and a type that overrides `child_lines` pays
// nothing. Reusing the value that the caller already holds needs the caller
// to know whether a type overrides `child_lines`, which Rust cannot tell
// without specialization or a new public method on `Row`. A `Row` method
// that receives that value, whose default reads the children field from it
// and which `child_lines` overrides replace, removes the second
// serialization when a profile shows that it matters.
pub(super) fn shown_lines<R: Serialize + ?Sized>(row: &R, children: Option<&str>) -> Vec<String> {
    let Some(children) = children else {
        return Vec::new();
    };
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(row)
    };
    let Ok(Value::Object(mut fields)) = value else {
        return Vec::new();
    };
    let Some(Value::Array(items)) = fields.remove(children) else {
        return Vec::new();
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(text) => Some(text),
            _ => None,
        })
        .collect::<Option<Vec<String>>>()
        .unwrap_or_default()
}
