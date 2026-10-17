//! Tests for the guide check of a batch that prints its failures among its
//! rows in input order.

use crumbkit::help::Guide;
use crumbkit::{Batch, Document, Error, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{check_guide, leaf, panic_message, shelf};
use crumbkit::Envelope;

/// An edition, whose field `schema` a failure document also has.
#[derive(Serialize, JsonSchema)]
struct Edition {
    /// The edition's id.
    id: u32,
    /// The id of the format that the edition follows.
    schema: String,
}

impl Row for Edition {
    const COLUMNS: &'static [&'static str] = &["id"];
}

/// The document of `shelf books show`, with its failures apart.
const EDITIONS: Document<Batch<Edition, Error>> =
    Document::new("shelf", "editions-show", 1, "The editions.").envelope(Envelope::Without);

#[test]
fn a_batch_in_input_order_whose_row_type_has_a_field_named_schema_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(EDITIONS.envelope(Envelope::WithoutInOrder)),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books show: the row type has a field named `schema`, and the document prints its failures among its rows in input order, so a reader could take a row for a failure document; rename the field"
    );
}

#[test]
fn a_batch_with_its_failures_apart_may_have_a_row_field_named_schema() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(EDITIONS),
        leaf(&["authors", "list"]),
    ]);

    check_guide(&shelf(), &GUIDE);
}

/// The document of `shelf books list`, a listing with an envelope for a
/// batch's failures in input order.
const LISTED_IN_ORDER: Document<Listing<Edition>> =
    Document::new("shelf", "books-list", 1, "The editions.").envelope(Envelope::KeysInOrder {
        type_key: "$type",
        data_key: "editions",
    });

#[test]
fn a_document_that_is_not_a_batch_fails_the_check_when_its_envelope_prints_failures_in_order() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).document(LISTED_IN_ORDER),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf books list: the document's envelope prints a batch's failures in input order, and the document is not a batch; choose Envelope::Keys or Envelope::Without"
    );
}
