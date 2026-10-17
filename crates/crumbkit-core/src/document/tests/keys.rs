//! Tests for a document that declares its own envelope keys.

use super::super::Document;
use crate::{Batch, Error, Listing};

use super::Moon;

const ENTRIES: Document<Listing<Moon>> =
    Document::new("tool", "get-entries", 1, "The entries.").with_keys("$type", "entries");

#[test]
fn a_document_uses_crumbkits_keys_and_prints_its_id_until_the_tool_declares_its_own() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");

    assert_eq!(document.type_key(), "schema");
    assert_eq!(document.data_key(), "data");
    assert_eq!(document.type_value(), None);
    assert!(!document.has_own_keys());
}

#[test]
fn declared_keys_replace_schema_and_data_and_keep_the_envelope() {
    assert_eq!(ENTRIES.type_key(), "$type");
    assert_eq!(ENTRIES.data_key(), "entries");
    assert_eq!(ENTRIES.type_value(), None);
    assert!(ENTRIES.has_own_keys());
    assert!(ENTRIES.has_envelope());
}

#[test]
fn with_type_sets_the_value_of_the_type_key_and_untyped_keeps_every_key() {
    let document = ENTRIES.with_type("tool/get-entries/v1").untyped();

    assert_eq!(document.type_value(), Some("tool/get-entries/v1"));
    assert_eq!(document.type_key(), "$type");
    assert_eq!(document.data_key(), "entries");
    assert!(document.has_own_keys());
}

#[test]
fn a_batch_with_its_own_keys_may_print_its_failures_in_input_order() {
    const ANSWERS: Document<Batch<Moon, Error>> =
        Document::new("tool", "get-report", 1, "The answers.").with_keys("$type", "answers");

    assert!(ANSWERS.failures_in_order().prints_failures_in_order());
}

#[test]
fn a_listing_with_its_own_keys_may_print_its_summary_at_the_top() {
    assert!(!ENTRIES.prints_summary_at_top());
    assert!(ENTRIES.summary_at_top().prints_summary_at_top());
    assert!(ENTRIES.summary_at_top().untyped().prints_summary_at_top());
}

#[test]
fn a_listing_with_its_own_keys_may_print_without_a_total() {
    assert!(ENTRIES.prints_total());
    assert!(!ENTRIES.without_total().prints_total());
    assert!(!ENTRIES.without_total().untyped().prints_total());
}

#[test]
#[should_panic(expected = "crumbkit's own envelope")]
fn a_listing_without_a_total_is_rejected_for_a_document_with_crumbkits_own_envelope() {
    let document: Document<Listing<Moon>> = Document::new("solar", "moons-list", 1, "Moons.");

    let _ = document.without_total();
}

#[test]
#[should_panic(expected = "crumbkit's own envelope")]
fn a_summary_at_the_top_is_rejected_for_a_document_with_crumbkits_own_envelope() {
    let document: Document<Listing<Moon>> = Document::new("solar", "moons-list", 1, "Moons.");

    let _ = document.summary_at_top();
}

#[test]
#[should_panic(expected = "the two keys differ")]
fn a_type_key_equal_to_the_data_key_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").with_keys("rows", "rows");
}

#[test]
#[should_panic(expected = "crumbkit writes")]
fn a_key_that_crumbkit_writes_beside_the_data_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").with_keys("$type", "total");
}

#[test]
#[should_panic(expected = "one non-empty line")]
fn an_empty_key_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").with_keys("", "rows");
}

#[test]
#[should_panic(expected = "without_envelope")]
fn declared_keys_are_rejected_for_a_document_without_the_envelope() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.")
        .without_envelope()
        .with_keys("$type", "rows");
}

#[test]
#[should_panic(expected = "with_keys")]
fn without_envelope_is_rejected_for_a_document_with_its_own_keys() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.")
        .with_keys("$type", "rows")
        .without_envelope();
}

#[test]
#[should_panic(expected = "with_keys")]
fn a_type_value_is_rejected_for_a_document_without_its_own_keys() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.").with_type("tool/get/v1");
}

#[test]
#[should_panic(expected = "one non-empty line")]
fn a_type_value_with_a_line_break_is_rejected() {
    let _: Document<()> = Document::new("tool", "get", 1, "Rows.")
        .with_keys("$type", "rows")
        .with_type("tool\nget");
}
