//! Tests for the tool record check of the form that `Row::LINE` declares
//! (ADR-0064).

use crumbkit::help::Help;
use crumbkit::{Batch, Bytes, Document, Error, Headed, Line, Row, Tool};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_help, leaf, panic_message, shelf};

/// A book that prints its own text line.
#[derive(Serialize, JsonSchema)]
struct LinedBook {
    /// The book's id.
    id: u32,
}

impl Row for LinedBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<Line<Self>> = Some(Line::new("book <ID>", |_| None));
}

/// A book whose text line form spans two lines.
#[derive(Serialize, JsonSchema)]
struct TwoLineBook {
    /// The book's id.
    id: u32,
}

impl Row for TwoLineBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<Line<Self>> = Some(Line::new("book\n<ID>", |_| None));
}

/// A book whose text line form is blank.
#[derive(Serialize, JsonSchema)]
struct BlankLineBook {
    /// The book's id.
    id: u32,
}

impl Row for BlankLineBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<Line<Self>> = Some(Line::new(" ", |_| None));
}

/// A book's file, which a headed batch prints under a header, and which
/// also declares a text line.
#[derive(Serialize, JsonSchema)]
struct LinedFile {
    /// The book's id.
    id: u32,
    /// The file's bytes.
    #[serde(flatten)]
    content: Bytes,
}

impl Row for LinedFile {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<Line<Self>> = Some(Line::new("file <ID>", |_| None));
}

impl Headed for LinedFile {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

#[test]
fn a_row_type_with_a_one_line_text_form_passes() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<LinedBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    check_help(&shelf(), &TOOL, &[]);
}

#[test]
fn check_tool_fails_on_a_text_line_form_that_spans_two_lines() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<TwoLineBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the row type's LINE is not one non-empty line, so help cannot show the form of each row's line on one line; write the form as one line, such as `<ID> <TITLE>`"
    );
}

#[test]
fn check_tool_fails_on_a_text_line_form_that_is_blank() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).document(Document::<Vec<BlankLineBook>>::new(
            "shelf",
            "books-list",
            1,
            "The test rows.",
        )),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]));

    assert!(
        panic_message(|| check_help(&shelf(), &TOOL, &[]))
            .unwrap()
            .contains("the row type's LINE is not one non-empty line")
    );
}

#[test]
fn check_tool_fails_on_a_headed_leaf_whose_row_type_declares_a_text_line() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[
            leaf(&["books", "list"]).document(
                Document::<Batch<LinedFile, Error>>::new(
                    "shelf",
                    "books-list",
                    1,
                    "The test rows.",
                )
                .headed(),
            ),
            leaf(&["books", "show"]),
            leaf(&["authors", "list"]),
        ]),
    );

    assert_eq!(
        panic_message(|| check_help(&shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books list: the row type declares LINE and the document is headed, so its text prints each row's bytes and never the line; remove LINE or the headed form"
    );
}
