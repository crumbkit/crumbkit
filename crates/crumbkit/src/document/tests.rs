//! Tests for document ids and their segment rules.

use std::cell::Cell;
use std::rc::Rc;

use super::Document;

#[test]
fn the_id_prints_as_component_name_and_version() {
    let document: Document<()> = Document::new("shelf", "books-list", 1);

    assert_eq!(document.to_string(), "shelf/books-list/v1");
}

#[test]
fn a_component_may_hold_digits_and_underscores_and_a_name_digits_and_dashes() {
    let document: Document<()> = Document::new("shelf_2", "books-list-2", 12);

    assert_eq!(document.to_string(), "shelf_2/books-list-2/v12");
}

#[test]
#[should_panic(expected = "component")]
fn an_empty_component_is_rejected() {
    let _: Document<()> = Document::new("", "books-list", 1);
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_an_uppercase_letter_is_rejected() {
    let _: Document<()> = Document::new("Shelf", "books-list", 1);
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_a_dash_is_rejected() {
    let _: Document<()> = Document::new("my-shelf", "books-list", 1);
}

#[test]
#[should_panic(expected = "component")]
fn a_component_with_a_letter_outside_ascii_is_rejected() {
    let _: Document<()> = Document::new("shélf", "books-list", 1);
}

#[test]
#[should_panic(expected = "name")]
fn an_empty_name_is_rejected() {
    let _: Document<()> = Document::new("shelf", "", 1);
}

#[test]
#[should_panic(expected = "name")]
fn a_name_with_an_underscore_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books_list", 1);
}

#[test]
#[should_panic(expected = "name")]
fn a_name_with_a_slash_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books/list", 1);
}

#[test]
#[should_panic(expected = "version")]
fn version_zero_is_rejected() {
    let _: Document<()> = Document::new("shelf", "books-list", 0);
}

#[test]
fn a_document_is_copy_send_and_sync_for_a_result_type_that_is_none_of_these() {
    fn assert_copy_send_sync<T: Copy + Send + Sync>(_: T) {}
    // `Rc<Cell<u8>>` is neither `Copy`, `Send` nor `Sync`.
    let document: Document<Rc<Cell<u8>>> = Document::new("shelf", "counter", 1);

    assert_copy_send_sync(document);
}
