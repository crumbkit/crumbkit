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

/// A series that a novel belongs to, which the schema names as a definition.
#[derive(Serialize, JsonSchema)]
struct Series {
    /// The series' name.
    name: String,
}

impl Series {
    /// Returns whether the series has no name, so the novel skips it.
    fn is_unnamed(&self) -> bool {
        self.name.is_empty()
    }
}

/// A row with columns that the row type skips when they are empty.
#[derive(Serialize, JsonSchema)]
struct Novel {
    /// The novel's id.
    id: u32,
    /// The novel's subtitle, which the row skips when it is empty.
    #[serde(skip_serializing_if = "String::is_empty")]
    subtitle: String,
    /// The novel's series, which the row skips when it has no name.
    #[serde(skip_serializing_if = "Series::is_unnamed")]
    series: Series,
    /// The novel's translator, which the row skips when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    translator: Option<String>,
    /// The novel's sequel, which the row skips when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    sequel: Option<Series>,
    /// The novel's notes, which the row skips when it has none and which
    /// is in neither column list.
    #[serde(skip_serializing_if = "String::is_empty")]
    notes: String,
}

impl Row for Novel {
    const COLUMNS: &'static [&'static str] = &["id", "subtitle", "series", "translator", "sequel"];
}

const NOVELS_LIST: Document<Listing<Novel>> =
    Document::new("shelf", "novels-list", 1, "One page of the novels.");

/// Returns whether the property schema `property` names `"null"` in its
/// `type`, or in the `type` of one of its `anyOf` members.
fn admits_null(property: &Value) -> bool {
    let named = |schema: &Value| match &schema["type"] {
        Value::String(name) => name == "null",
        Value::Array(names) => names.contains(&json!("null")),
        _ => false,
    };
    named(property)
        || property["anyOf"]
            .as_array()
            .is_some_and(|members| members.iter().any(named))
}

#[test]
fn a_column_that_the_row_skips_when_it_is_empty_admits_null_and_stays_optional() {
    let schema = document_schema(NOVELS_LIST).to_value();
    let novel = &schema["$defs"]["Novel"];

    assert_eq!(novel["required"], json!(["id"]));
    assert_eq!(
        novel["properties"]["subtitle"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        novel["properties"]["subtitle"]["description"],
        "The novel's subtitle, which the row skips when it is empty."
    );
}

#[test]
fn a_stream_schema_lets_a_column_that_the_row_skips_when_it_is_empty_admit_null() {
    const NOVELS_WATCH: Document<Stream<Novel>> =
        Document::new("shelf", "novels-watch", 1, "Each novel as it is shelved.");

    let schema = stream_schema(NOVELS_WATCH).to_value();
    let novel = &schema["$defs"]["Novel"];

    assert_eq!(novel["required"], json!(["id"]));
    assert_eq!(
        novel["properties"]["subtitle"]["type"],
        json!(["string", "null"])
    );
}

#[test]
fn a_skipped_column_whose_property_names_a_definition_admits_null_through_any_of() {
    let schema = document_schema(NOVELS_LIST).to_value();
    let series = &schema["$defs"]["Novel"]["properties"]["series"];

    assert_eq!(
        series,
        &json!({
            "anyOf": [{ "$ref": "#/$defs/Series" }, { "type": "null" }],
            "description": "The novel's series, which the row skips when it has no name.",
        })
    );
}

#[test]
fn a_skipped_column_that_already_admits_null_keeps_its_property() {
    let schema = document_schema(NOVELS_LIST).to_value();
    let whole = document_schema(NOVELS_LIST.envelope(Envelope::Without)).to_value();
    let properties = &schema["$defs"]["Novel"]["properties"];
    let whole_properties = &whole["$defs"]["Novel"]["properties"];

    assert_eq!(properties["translator"], whole_properties["translator"]);
    assert_eq!(properties["sequel"], whole_properties["sequel"]);
    assert_eq!(properties["translator"]["type"], json!(["string", "null"]));
}

#[test]
fn a_required_column_and_a_skipped_field_outside_the_columns_keep_their_types() {
    let schema = document_schema(NOVELS_LIST).to_value();
    let properties = &schema["$defs"]["Novel"]["properties"];

    assert_eq!(properties["id"]["type"], "integer");
    assert_eq!(properties["notes"]["type"], "string");
}

#[test]
fn a_compact_row_with_null_for_a_column_that_the_row_skipped_matches_the_property_type() {
    let novel = Novel {
        id: 3,
        subtitle: String::new(),
        series: Series {
            name: String::new(),
        },
        translator: None,
        sequel: None,
        notes: String::new(),
    };

    let schema = document_schema(NOVELS_LIST).to_value();
    let definition = &schema["$defs"]["Novel"];
    // The compact row holds each column, and null for a column that the
    // row skipped, as `--json` prints it.
    let whole = serde_json::to_value(&novel).unwrap();
    let compact: serde_json::Map<String, Value> = Novel::COLUMNS
        .iter()
        .map(|&name| {
            (
                name.to_owned(),
                whole.get(name).cloned().unwrap_or(Value::Null),
            )
        })
        .collect();

    assert_eq!(whole.get("subtitle"), None);
    for (name, value) in &compact {
        if value.is_null() {
            assert!(
                admits_null(&definition["properties"][name]),
                "{name} does not admit null"
            );
        }
    }
    for name in definition["required"].as_array().unwrap() {
        assert!(compact.contains_key(name.as_str().unwrap()), "{name}");
    }
}

/// A row that is an article or a note, with a skipped column in one variant.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Entry {
    /// An article.
    Article {
        /// The entry's id.
        id: u32,
        /// The article's byline, which the row skips when it is empty.
        #[serde(skip_serializing_if = "String::is_empty")]
        byline: String,
    },
    /// A note.
    Note {
        /// The entry's id.
        id: u32,
    },
}

impl Row for Entry {
    const COLUMNS: &'static [&'static str] = &["kind", "id", "byline"];
}

#[test]
fn a_skipped_column_in_an_enum_variant_admits_null_in_that_variant() {
    const ENTRIES: Document<Listing<Entry>> =
        Document::new("shelf", "entries-list", 1, "One page of the entries.");

    let entries = [
        Entry::Article {
            id: 1,
            byline: String::new(),
        },
        Entry::Note { id: 2 },
    ];

    let schema = document_schema(ENTRIES).to_value();
    let variants = schema["$defs"]["Entry"]["oneOf"].as_array().unwrap();

    assert_eq!(variants[0]["required"], json!(["kind", "id"]));
    assert_eq!(
        variants[0]["properties"]["byline"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(variants[1]["properties"]["byline"], json!({"type": "null"}));
    for (entry, variant) in entries.iter().zip(variants) {
        let whole = serde_json::to_value(entry).unwrap();
        assert_eq!(whole.get("byline"), None);
        assert!(admits_null(&variant["properties"]["byline"]));
    }
}

/// A service whose components print under it in text and sit after the
/// columns in compact JSON.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: String,
    /// Where the service runs.
    region: String,
    /// The parts that the service runs on.
    components: Vec<String>,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("components");
}

#[test]
fn a_schema_requires_the_children_field_beside_the_columns_because_compact_json_holds_it() {
    const SERVICES: Document<Listing<Service>> =
        Document::new("ops", "services-list", 1, "One page of the services.");

    let schema = document_schema(SERVICES).to_value();

    assert_eq!(
        required_of(&schema, "Service"),
        json!(["name", "components"])
    );
}
