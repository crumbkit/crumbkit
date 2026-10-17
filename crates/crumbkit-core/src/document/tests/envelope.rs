//! Tests for the envelope that a document prints its result in.

use super::super::{Document, Envelope};

#[test]
fn a_new_document_has_crumbkits_envelope_which_wraps_the_result() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");
    let envelope = document.get_envelope();

    assert_eq!(envelope, Envelope::Crumbkit);
    assert!(envelope.wraps());
}

#[test]
fn a_document_without_its_envelope_keeps_its_id_and_prints_the_result_alone() {
    let document: Document<()> =
        Document::new("shelf", "books-list", 3, "Books.").envelope(Envelope::Without);

    assert_eq!(document.untyped().get_envelope(), Envelope::Without);
    assert!(!document.get_envelope().wraps());
    assert_eq!(document.to_string(), "shelf/books-list/v3");
}
