//! Tests for the guide check of the failure field names that a tool takes
//! from another system.

use crumbkit::help::{Guide, Leaf};

use super::{check_guide, leaf, panic_message, shelf};

/// The leaves of every command of `shelf`.
const LEAVES: &[Leaf] = &[
    leaf(&["books", "list"]),
    leaf(&["books", "show"]),
    leaf(&["authors", "list"]),
];

#[test]
fn field_names_that_break_the_rule_and_hold_printable_text_pass() {
    const GUIDE: Guide =
        Guide::new()
            .leaves(LEAVES)
            .field_names(&["requestId", "Content-Type", "x.y"]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn an_empty_a_spaced_a_repeated_and_a_rule_following_field_name_fail_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES).field_names(&[
        "",
        "request id",
        "line\nbreak",
        "requestId",
        "requestId",
        "request_id",
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the field name \"\" is empty, so no failure field can have it\n\
         the field name \"request id\" holds whitespace or a control character, which a reader cannot tell apart from the text around it\n\
         the field name \"line\\nbreak\" holds whitespace or a control character, which a reader cannot tell apart from the text around it\n\
         the field name \"requestId\" is listed more than once\n\
         the field name \"request_id\" follows crumbkit's field name rule already, so it needs no listing"
    );
}
