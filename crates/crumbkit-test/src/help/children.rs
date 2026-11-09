//! The tool record check of the field that `Row::CHILDREN` names (ADR-0120) and
//! of the text that `Children::help` sets (ADR-0137).

use crumbkit::Children;
use crumbkit::help::{Leaf, LeafDocument};

/// Adds a problem to `problems` for each way in which the field that
/// `Row::CHILDREN` names for the leaf at `text` breaks the rules: the
/// field is not a property of the row type's JSON Schema, has no doc
/// comment, belongs to a headed document, whose text never prints the
/// child lines, or is also one of the row type's columns.
pub(super) fn check_children(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    let Some(document) = leaf.get_document() else {
        return;
    };
    check_children_help(text, document, problems);
    let Some(column) = document.children_column() else {
        return;
    };
    let field = column.name();
    if !column.is_property() {
        problems.push(format!(
            "{text}: the children field `{field}` is not a field of the row type, so the JSON holds no children for the lines that the text prints under each row; name a field of the row type in `Row::CHILDREN`"
        ));
    } else if column.description().is_none() {
        problems.push(format!(
            "{text}: the children field `{field}` has no description, because the field `{field}` of the row type has no doc comment; write a doc comment on the field"
        ));
    }
    if document.document().is_headed() {
        problems.push(format!(
            "{text}: the row type declares CHILDREN and the document is headed, so its text prints each row's bytes and never the child lines; remove CHILDREN or the headed form"
        ));
        return;
    }
    let columns = document.column_descriptions();
    if let Some(also) = columns.iter().find(|also| also.name() == field) {
        let list = if also.is_long() {
            "LONG_COLUMNS"
        } else {
            "COLUMNS"
        };
        problems.push(format!(
            "{text}: the children field `{field}` is also one of the row type's {list}, so the table prints the children as a cell of JSON and again as lines under the row; remove `{field}` from {list}"
        ));
    }
}

/// Adds a problem to `problems` for each way in which the text that
/// `Children::help` sets for the leaf at `text` breaks the rules: it is
/// empty or holds a control character, such as a line feed or an escape
/// code, so help cannot print it as one line, or it does not end with a full
/// stop.
fn check_children_help(text: &str, document: &LeafDocument, problems: &mut Vec<String>) {
    let Some(help) = document.children().and_then(Children::get_help) else {
        return;
    };
    if help.trim().is_empty() || help.chars().any(char::is_control) {
        problems.push(format!(
            "{text}: the row type's `Children::help` is empty or holds a control character, such as a line feed, so help cannot print it on the one Children: line; write the text as one line"
        ));
    }
    if !help.ends_with('.') {
        problems.push(format!(
            "{text}: the row type's `Children::help` does not end with a full stop, as every other line of help does; end the text with `.`"
        ));
    }
}
