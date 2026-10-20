//! The guide check of the field that `Row::CHILDREN` names (ADR-0120).

use crumbkit::help::Leaf;

/// Adds a problem to `problems` for each way in which the field that
/// `Row::CHILDREN` names for the leaf at `text` breaks the rules: the
/// field is not a property of the row type's JSON Schema, has no doc
/// comment, belongs to a headed document, whose text never prints the
/// child lines, or is also one of the row type's columns.
pub(super) fn check_children(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
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
