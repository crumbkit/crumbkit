//! Tests for the tool record check of a leaf's notes.

use crumbkit::Tool;
use crumbkit::help::Help;

use super::{check_help, leaf, panic_message, shelf};

#[test]
fn a_leaf_with_one_line_notes_passes() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).notes(&["Books on loan are left out.", "Ids are stable."]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    check_help(&shelf(), &TOOL, &[]);
}

#[test]
fn check_help_fails_a_note_that_holds_a_line_break() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).notes(&["Books on loan\nare left out."]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the note `Books on loan\\nare left out.` holds a line break, so help cannot print it on one indented line; split it into one note for each line"
    );
}

#[test]
fn check_help_fails_an_empty_note() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).notes(&["Ids are stable.", " "]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: a note is empty, so help prints a blank line under `Notes:`; remove it"
    );
}
