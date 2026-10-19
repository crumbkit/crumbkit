//! Tests for document ids and their segment rules.

use std::cell::Cell;
use std::rc::Rc;

use super::Document;

mod envelope;

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
fn a_new_document_may_refuse_its_first_answer() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");

    assert!(!document.always_prints_first());
}

#[test]
fn an_untyped_document_keeps_its_first_answer_setting() {
    let document: Document<u32> =
        Document::new("shelf", "books-show", 1, "Books.").always_print_first();

    let untyped = document.untyped();

    assert!(untyped.always_prints_first());
}

#[test]
fn a_document_has_no_empty_note_until_the_tool_sets_one() {
    let document: Document<()> = Document::new("shelf", "books-list", 1, "Books.");

    assert_eq!(document.empty_note(), None);
}

#[test]
fn when_empty_sets_the_note_and_untyped_keeps_it() {
    let document: Document<()> =
        Document::new("shelf", "books-list", 1, "Books.").when_empty("the shelf holds no books");

    assert_eq!(document.empty_note(), Some("the shelf holds no books"));
    assert_eq!(
        document.untyped().empty_note(),
        Some("the shelf holds no books")
    );
}

#[test]
#[should_panic(expected = "empty note")]
fn an_empty_note_with_a_line_break_is_rejected() {
    let _: Document<()> =
        Document::new("shelf", "books-list", 1, "Books.").when_empty("no books\nat all");
}

/// A file of the headed batches in these tests, which holds bytes unless it
/// holds a record.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Page {
    /// The page's name.
    name: &'static str,
    #[serde(flatten)]
    content: Option<crate::Bytes>,
}

impl crate::Row for Page {
    const COLUMNS: &'static [&'static str] = &["name"];
}

impl crate::Headed for Page {
    fn bytes(&self) -> Option<&crate::Bytes> {
        self.content.as_ref()
    }
}

/// The document of the headed batches in these tests.
const PAGES: Document<crate::Batch<Page, crate::Error>> =
    Document::new("book", "pages-show", 1, "Pages.");

#[test]
fn a_batch_prints_as_a_table_until_the_tool_marks_its_document_headed() {
    assert!(!PAGES.is_headed());
    assert!(PAGES.headed_bytes().is_none());
    assert!(PAGES.headed().is_headed());
    assert!(PAGES.headed().untyped().is_headed());
    assert!(!PAGES.untyped().is_headed());
}

#[test]
fn a_headed_document_reads_the_bytes_of_each_row_by_its_position() {
    let mut batch = crate::Batch::new(&["pages", "show"], "names", &["one", "two", "three"]);
    batch.push(Ok(Page {
        name: "one",
        content: Some(crate::Bytes::from("first\n".to_owned())),
    }));
    batch.push(Ok(Page {
        name: "two",
        content: None,
    }));
    let bytes_of = PAGES.headed().headed_bytes().unwrap();

    assert_eq!(bytes_of(&batch, 0), Some(&b"first\n"[..]));
    assert_eq!(bytes_of(&batch, 1), None);
    assert_eq!(bytes_of(&batch, 2), None);
}
