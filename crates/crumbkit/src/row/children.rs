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
