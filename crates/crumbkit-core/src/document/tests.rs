//! Tests for document ids and their segment rules.

use std::cell::Cell;
use std::rc::Rc;

use super::Document;

#[test]
fn the_id_prints_as_component_name_and_version() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");

    assert_eq!(document.to_string(), "shelf/books-list/v1");
}

#[test]
fn a_component_may_hold_digits_and_underscores_and_a_name_digits_and_dashes() {
    let document: Document<()> = Document::new("shelf_2", "books-list-2", 12, "Books.");

    assert_eq!(document.to_string(), "shelf_2/books-list-2/v12");
}

#[test]
#[should_panic(expected = "component")]
fn an_empty_component_is_rejected() {
    let _: Document<()> = Document::new("", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_an_uppercase_letter_is_rejected() {
    let _: Document<()> = Document::new("Shelf", "books-list", 1, "Books.");
}

#[test]
fn a_component_may_hold_dots_and_dashes_as_a_domain_does() {
    let domain: Document<()> = Document::new("example.dev", "books-list", 1, "Books.");
    let dashed: Document<()> = Document::new("my-shelf.example_2", "books-list", 1, "Books.");

    assert_eq!(domain.to_string(), "example.dev/books-list/v1");
    assert_eq!(dashed.to_string(), "my-shelf.example_2/books-list/v1");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_that_starts_with_a_dot_is_rejected() {
    let _: Document<()> = Document::new(".dev", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_that_ends_with_a_dot_is_rejected() {
    let _: Document<()> = Document::new("example.", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_two_dots_in_a_row_is_rejected() {
    let _: Document<()> = Document::new("example..dev", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_a_slash_is_rejected() {
    let _: Document<()> = Document::new("example/dev", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_a_letter_outside_ascii_is_rejected() {
    let _: Document<()> = Document::new("shélf", "books-list", 1, "Books.");
}

#[test]
#[should_panic(expected = "name")]
fn an_empty_name_is_rejected() {
    let _: Document<()> = Document::new("shelf", "", 1, "Books.");
}

#[test]
#[should_panic(expected = "name")]
fn a_name_with_an_underscore_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books_list", 1, "Books.");
}

#[test]
#[should_panic(expected = "name")]
fn a_name_with_a_slash_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books/list", 1, "Books.");
}

#[test]
#[should_panic(expected = "version")]
fn version_zero_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books-list", 0, "Books.");
}

#[test]
fn a_document_is_copy_send_and_sync_for_a_result_type_that_is_none_of_these() {
    fn assert_copy_send_sync<T: Copy + Send + Sync>(_: T) {}
    // `Rc<Cell<u8>>` is neither `Copy`, `Send` nor `Sync`.
    let document: Document<Rc<Cell<u8>>> = Document::new("shelf", "counter", 1, "Books.");

    assert_copy_send_sync(document);
}

#[test]
fn the_getters_return_the_parts_of_the_id_and_the_description() {
    let document: Document<()> = Document::new("shelf", "books-list", 2, "One page of books.");

    assert_eq!(document.component(), "shelf");
    assert_eq!(document.name(), "books-list");
    assert_eq!(document.version(), 2);
    assert_eq!(document.description(), "One page of books.");
}

#[test]
#[should_panic(expected = "description")]
fn an_empty_description_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books-list", 1, "");
}

#[test]
#[should_panic(expected = "description")]
fn a_description_of_more_than_one_line_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books-list", 1, "One page.\nOf books.");
}

#[test]
fn an_untyped_document_keeps_the_id_and_the_description() {
    let document: Document<Vec<u32>> = Document::new("shelf", "books-list", 1, "Books.");

    let untyped: Document<()> = document.untyped();

    assert_eq!(untyped.to_string(), "shelf/books-list/v1");
    assert_eq!(untyped.description(), "Books.");
}

#[test]
fn a_new_document_has_crumbkits_envelope() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");

    assert!(document.has_envelope());
}

#[test]
fn a_document_without_its_envelope_keeps_its_id_and_reports_no_envelope() {
    let document: Document<()> =
        Document::new("shelf", "books-list", 3, "Books.").without_envelope();

    assert!(!document.has_envelope());
    assert_eq!(document.to_string(), "shelf/books-list/v3");
}

#[test]
fn an_untyped_document_keeps_the_choice_of_envelope() {
    let document: Document<Vec<u32>> =
        Document::new("shelf", "books-list", 1, "Books.").without_envelope();

    assert!(!document.untyped().has_envelope());
}
