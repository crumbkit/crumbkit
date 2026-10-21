//! The guide check of the field that `Row::CHILDREN` names (ADR-0120) and
//! of the text that `Row::CHILDREN_HELP` sets (ADR-0137).

use crumbkit::help::Leaf;

/// Adds a problem to `problems` for each way in which the field that
/// `Row::CHILDREN` names for the leaf at `text` breaks the rules: the
/// field is not a property of the row type's JSON Schema, has no doc
/// comment, belongs to a headed document, whose text never prints the
/// child lines, or is also one of the row type's columns.
pub(super) fn check_children(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    check_children_help(text, leaf, problems);
    let Some(column) = leaf.children_column() else {
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
    let headed = leaf
        .get_document()
        .is_some_and(|document| document.is_headed());
    if headed {
        problems.push(format!(
            "{text}: the row type declares CHILDREN and the document is headed, so its text prints each row's bytes and never the child lines; remove CHILDREN or the headed form"
        ));
        return;
    }
    let columns = leaf.column_descriptions();
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
/// `Row::CHILDREN_HELP` sets for the leaf at `text` breaks the rules: it is
/// empty or holds a control character, such as a line feed or an escape
/// code, so help cannot print it as one line; it does not end with a full
/// stop; or the row type names no children field, so help never prints it.
fn check_children_help(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    let Some(help) = leaf.children_help() else {
        return;
    };
    if help.trim().is_empty() || help.chars().any(char::is_control) {
        problems.push(format!(
            "{text}: the row type's CHILDREN_HELP is empty or holds a control character, such as a line feed, so help cannot print it on the one Children: line; write the text as one line"
        ));
    }
    if !help.ends_with('.') {
        problems.push(format!(
            "{text}: the row type's CHILDREN_HELP does not end with a full stop, as every other line of help does; end the text with `.`"
        ));
    }
    if leaf.children().is_none() {
        problems.push(format!(
            "{text}: the row type sets CHILDREN_HELP and no CHILDREN, so its rows print no child lines and help never prints the text; name the children field in `Row::CHILDREN` or remove CHILDREN_HELP"
        ));
    }
}
