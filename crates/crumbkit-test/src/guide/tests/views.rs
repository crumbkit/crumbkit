//! Tests for the guide check of the view flags that a leaf offers: the
//! flags that an example types, and the document that a leaf with effects
//! declares so that `parse` can refuse a flag before the handler runs.

use clap::Args as _;
use crumbkit::help::{Effects, Example, Guide};
use crumbkit::{Document, Output, Prose};

use super::{check_guide, leaf, panic_message, shelf};

/// The prose document of `shelf books list` in these tests.
const COUNT: Document<Prose<String>> = Document::new("shelf", "count", 1, "The count.");

#[test]
fn check_guide_fails_on_an_example_that_types_a_view_flag_its_leaf_does_not_offer() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"])
            .examples(&[Example::new(&["--long"], "Count the books at length")])
            .document(COUNT),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&Output::augment_args(shelf()), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the example `shelf books list --long` types --long, which the command does not offer, so crumbkit refuses it"
    );
}

#[test]
fn check_guide_fails_on_a_leaf_that_declares_a_change_and_no_document() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).effects(Effects::additive()),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream"
    );
}

#[test]
fn a_read_only_leaf_without_a_document_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).effects(Effects::read_only()),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE);
}

#[test]
fn a_changing_leaf_that_passes_on_a_programs_exit_code_passes_without_a_document() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"])
            .effects(Effects::replacing())
            .passes_on_exit("the editor"),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE);
}

/// A book's file whose bytes sit under `content` in place of the top level
/// of the row, so its JSON has no `text` or `base64` field.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct NestedFile {
    /// The book's title.
    title: String,
    /// The file's bytes.
    content: crumbkit::Bytes,
}

impl crumbkit::Row for NestedFile {
    const COLUMNS: &'static [&'static str] = &["title"];
}

impl crumbkit::Headed for NestedFile {
    fn bytes(&self) -> Option<&crumbkit::Bytes> {
        Some(&self.content)
    }
}

#[test]
fn check_guide_fails_on_a_headed_leaf_whose_row_does_not_flatten_its_bytes() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(
            Document::<crumbkit::Batch<NestedFile, crumbkit::Error>>::new(
                "shelf",
                "books-files",
                1,
                "The books' files.",
            )
            .headed(),
        ),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the field `text` is not at the top level of the row type, so the JSON of the headed document does not hold the bytes beside the other fields; mark the `Bytes` field `#[serde(flatten)]`\n\
         shelf books list: the field `base64` is not at the top level of the row type, so the JSON of the headed document does not hold the bytes beside the other fields; mark the `Bytes` field `#[serde(flatten)]`"
    );
}
