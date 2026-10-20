//! Tests for the `required` list of a row in a document whose JSON prints
//! the row's columns without `--full`.

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use crate::schemas::{document_schema, stream_schema};
use crate::{Batch, Bytes, Document, Envelope, Error, Headed, Listing, Row, Stream};

/// A row with one field that is in neither column list.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The book's ISBN, which only `--json --full` prints.
    isbn: String,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
}

/// A row that `--json` prints with every field.
#[derive(Serialize, JsonSchema)]
struct WholeBook {
    /// The book's id.
    id: u32,
    /// The book's ISBN.
    isbn: String,
}

impl Row for WholeBook {
    const COLUMNS: &'static [&'static str] = &["id"];
    const WHOLE: bool = true;
}

/// A file and its contents, for a headed batch.
#[derive(Serialize, JsonSchema)]
struct File {
    /// The file's path.
    path: String,
    /// The file's size in bytes.
    size: u64,
    #[serde(flatten)]
    content: Bytes,
}

impl Row for File {
    const COLUMNS: &'static [&'static str] = &["path"];
}

impl Headed for File {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

/// A row that is a book or a film, with different fields in each variant.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Item {
    /// A book.
    Book {
        /// The item's id.
        id: u32,
        /// The book's title.
        title: String,
        /// The book's ISBN.
        isbn: String,
    },
    /// A film.
    Film {
        /// The item's id.
        id: u32,
        /// The film's length in minutes.
        minutes: u32,
    },
}

impl Row for Item {
    const COLUMNS: &'static [&'static str] = &["kind", "id", "title"];
}

/// A summary that holds a book, so the row's definition serves two fields.
#[derive(Serialize, JsonSchema)]
struct Newest {
    /// The newest book.
    newest: Book,
}

impl std::fmt::Display for Newest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.newest.title)
    }
}

const BOOKS_LIST: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");

/// Returns the `required` list of `name` under the `$defs` of `schema`.
fn required_of(schema: &Value, name: &str) -> Value {
    schema["$defs"][name]["required"].clone()
}

#[test]
fn a_listing_schema_requires_each_column_of_its_row_and_leaves_a_field_outside_the_columns_optional()
 {
    let schema = document_schema(BOOKS_LIST).to_value();

    assert_eq!(
        schema["properties"]["data"]["items"]["$ref"],
        "#/$defs/Book"
    );
    assert_eq!(required_of(&schema, "Book"), json!(["id", "title"]));
    assert_eq!(
        schema["$defs"]["Book"]["properties"]["isbn"]["type"],
        "string"
    );
}

#[test]
fn a_stream_schema_requires_each_column_of_its_row_and_leaves_a_field_outside_the_columns_optional()
{
    const BOOKS_WATCH: Document<Stream<Book>> =
        Document::new("shelf", "books-watch", 1, "Each book as it is shelved.");

    let schema = stream_schema(BOOKS_WATCH).to_value();

    assert_eq!(required_of(&schema, "Book"), json!(["id", "title"]));
}

#[test]
fn a_single_row_schema_requires_each_column_of_its_row_and_leaves_a_field_outside_the_columns_optional()
 {
    const BOOK: Document<Book> = Document::new("shelf", "book", 1, "One book.");

    let schema = document_schema(BOOK).to_value();

    assert_eq!(required_of(&schema, "Book"), json!(["id", "title"]));
}

#[test]
fn a_list_and_a_batch_that_is_not_headed_require_only_the_columns_of_their_row() {
    const BOOKS_PURGE: Document<Vec<Book>> =
        Document::new("shelf", "books-purge", 1, "The books that were removed.");
    const BOOKS_SHOW: Document<Batch<Book, Error>> =
        Document::new("shelf", "books-show", 1, "The books for the ids given.");

    for schema in [document_schema(BOOKS_PURGE), document_schema(BOOKS_SHOW)] {
        assert_eq!(
            required_of(&schema.to_value(), "Book"),
            json!(["id", "title"])
        );
    }
}

#[test]
fn a_row_that_prints_whole_keeps_every_field_required() {
    const WHOLE: Document<Listing<WholeBook>> = Document::new("shelf", "whole", 1, "Whole books.");

    let schema = document_schema(WHOLE).to_value();

    assert_eq!(required_of(&schema, "WholeBook"), json!(["id", "isbn"]));
}

#[test]
fn a_headed_batch_keeps_every_field_required() {
    const FILES: Document<Batch<File, Error>> =
        Document::new("files", "files-show", 1, "The files given.").headed();

    let schema = document_schema(FILES).to_value();
    let required = schema["$defs"]["File"]["required"].as_array().unwrap();

    assert!(required.contains(&json!("path")));
    assert!(required.contains(&json!("size")));
}

#[test]
fn a_document_without_the_envelope_keeps_every_field_required() {
    let schema = document_schema(BOOKS_LIST.envelope(Envelope::Without)).to_value();

    assert_eq!(required_of(&schema, "Book"), json!(["id", "title", "isbn"]));
}

#[test]
fn an_enum_row_has_the_required_list_of_each_variant_cut_to_the_columns() {
    const ITEMS: Document<Listing<Item>> =
        Document::new("shelf", "items-list", 1, "One page of the items.");

    let schema = document_schema(ITEMS).to_value();
    let variants = schema["$defs"]["Item"]["oneOf"].as_array().unwrap();

    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0]["required"], json!(["kind", "id", "title"]));
    assert_eq!(variants[1]["required"], json!(["kind", "id"]));
}

#[test]
fn a_compact_row_of_an_enum_with_null_for_a_column_its_variant_lacks_matches_its_variant() {
    const ITEMS: Document<Listing<Item>> =
        Document::new("shelf", "items-list", 1, "One page of the items.");
    let items = [
        Item::Book {
            id: 1,
            title: "Emma".to_owned(),
            isbn: "9780141439587".to_owned(),
        },
        Item::Film { id: 7, minutes: 90 },
    ];

    let schema = document_schema(ITEMS).to_value();
    let variants = schema["$defs"]["Item"]["oneOf"].as_array().unwrap();

    assert_ne!(
        schema["$defs"]["Item"].get("additionalProperties"),
        Some(&json!(false))
    );
    for (item, variant) in items.iter().zip(variants) {
        // The compact row holds each column, and null for a column that the
        // item lacks, as `--json` prints it.
        let whole = serde_json::to_value(item).unwrap();
        let compact: serde_json::Map<String, Value> = Item::COLUMNS
            .iter()
            .map(|&name| {
                (
                    name.to_owned(),
                    whole.get(name).cloned().unwrap_or(Value::Null),
                )
            })
            .collect();
        assert_eq!(variant["properties"]["kind"]["const"], compact["kind"]);
        for name in variant["required"].as_array().unwrap() {
            assert!(compact.contains_key(name.as_str().unwrap()), "{name}");
        }
        assert_ne!(variant.get("additionalProperties"), Some(&json!(false)));
    }
}

#[test]
fn a_row_whose_definition_serves_another_field_is_cut_in_an_inline_copy() {
    const NEWEST: Document<Listing<Book, Newest>> =
        Document::new("shelf", "newest", 1, "The books and the newest one.");

    let schema = document_schema(NEWEST).to_value();
    let items = &schema["properties"]["data"]["items"];

    assert_eq!(items.get("$ref"), None);
    assert_eq!(items["required"], json!(["id", "title"]));
    assert_eq!(items["properties"]["isbn"]["type"], "string");
    assert_eq!(required_of(&schema, "Book"), json!(["id", "title", "isbn"]));
}
