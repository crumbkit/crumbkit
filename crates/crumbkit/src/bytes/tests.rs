//! Tests for bytes results and their JSON form. crumbkit's tests check how
//! `finish_to` prints them.

use proptest::prelude::*;
use schemars::{JsonSchema, SchemaGenerator};
use serde::Serialize;
use serde_json::json;

use super::{Base64, Bytes};
use crate::Rows;
use crate::row::Sealed;
use crate::shown::TextCells;
use crate::{NoRow, Row, Shape};

/// Returns the base64 text of `bytes`.
fn base64(bytes: &[u8]) -> String {
    Base64(bytes).to_string()
}

#[test]
fn the_encoder_gives_the_test_vectors_of_rfc_4648() {
    let vectors = [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ];

    for (input, encoded) in vectors {
        assert_eq!(base64(input.as_bytes()), encoded, "input {input:?}");
    }
}

#[test]
fn the_encoder_uses_the_standard_alphabet_for_every_byte_value() {
    let every: Vec<u8> = (0..=255).collect();

    assert_eq!(
        base64(&every),
        "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w=="
    );
}

proptest! {
    /// The encoder writes its text in pieces, so this checks that the
    /// pieces join up: three bytes are always four characters, so the text
    /// of two inputs whose first has a multiple of three bytes is the text
    /// of each joined.
    #[test]
    fn the_text_of_joined_inputs_is_the_joined_text_of_each(
        first in proptest::collection::vec(any::<u8>(), 0..3000),
        second in proptest::collection::vec(any::<u8>(), 0..3000),
    ) {
        let whole = first.len() / 3 * 3;
        let first = first.get(..whole).unwrap_or_default();
        let joined: Vec<u8> = first.iter().chain(&second).copied().collect();

        prop_assert_eq!(base64(&joined), base64(first) + &base64(&second));
        prop_assert_eq!(base64(&joined).len(), joined.len().div_ceil(3) * 4);
    }
}

#[test]
fn utf8_bytes_serialize_as_text() {
    let bytes = Bytes::from("North and South\n\u{1b}[31m".to_owned());

    assert_eq!(
        serde_json::to_value(&bytes).unwrap(),
        json!({"text": "North and South\n\u{1b}[31m"})
    );
}

#[test]
fn bytes_that_are_not_utf8_serialize_as_base64() {
    let bytes = Bytes::new(vec![0x66, 0x6f, 0xff]);

    assert_eq!(
        serde_json::to_value(&bytes).unwrap(),
        json!({"base64": "Zm//"})
    );
}

#[test]
fn empty_bytes_serialize_as_empty_text() {
    assert_eq!(
        serde_json::to_value(Bytes::new(Vec::new())).unwrap(),
        json!({"text": ""})
    );
}

#[test]
fn a_bytes_result_has_no_rows_and_returns_its_bytes() {
    let bytes = Bytes::from(vec![1, 2, 3]);

    assert_eq!(bytes.rows().len(), 0);
    assert_eq!(Sealed::bytes(&bytes), Some(&[1_u8, 2, 3][..]));
    assert_eq!(bytes.as_slice(), &[1, 2, 3]);
    assert_eq!(<Bytes as Sealed>::SHAPE, Shape::Bytes);
    assert_eq!(<<Bytes as Sealed>::Row as Row>::COLUMNS, NoRow::COLUMNS);
}

#[test]
fn the_schema_is_an_object_that_holds_exactly_one_of_text_and_base64() {
    let schema = schemars::schema_for!(Bytes);

    assert_eq!(schema.get("type"), Some(&json!("object")));
    assert_eq!(schema.get("additionalProperties"), Some(&json!(false)));
    assert_eq!(
        schema.get("oneOf"),
        Some(&json!([{"required": ["text"]}, {"required": ["base64"]}]))
    );
    assert_eq!(
        schema.get("properties").unwrap()["base64"]["contentEncoding"],
        json!("base64")
    );
}

/// A row that holds a file's name and its bytes.
#[derive(Serialize, JsonSchema)]
struct File {
    /// The file's name.
    name: &'static str,
    /// The file's bytes.
    content: Bytes,
}

#[test]
fn a_row_field_of_bytes_serializes_in_the_same_form_and_names_the_bytes_definition() {
    let file = File {
        name: "cover.png",
        content: Bytes::new(vec![0x89, 0x50]),
    };
    let mut generator = SchemaGenerator::default();
    let _ = generator.subschema_for::<File>();

    assert_eq!(
        serde_json::to_value(&file).unwrap(),
        json!({"name": "cover.png", "content": {"base64": "iVA="}})
    );
    assert!(generator.definitions().contains_key("Bytes"));
}

/// A file whose bytes sit at the top level of the row, as `text` or
/// `base64`.
#[derive(Serialize, JsonSchema)]
struct Flat {
    /// The file's name.
    name: &'static str,
    #[serde(flatten)]
    content: Bytes,
}

/// A file that may hold a parsed record in place of its bytes.
#[derive(Serialize, JsonSchema)]
struct MaybeFlat {
    /// The file's name.
    name: &'static str,
    #[serde(flatten)]
    content: Option<Bytes>,
}

#[test]
fn a_flattened_bytes_field_puts_text_or_base64_at_the_top_level_of_the_row() {
    let text = Flat {
        name: "a.txt",
        content: Bytes::from("one\n".to_owned()),
    };
    let binary = Flat {
        name: "a.png",
        content: Bytes::new(vec![0x89, 0x50]),
    };

    assert_eq!(
        serde_json::to_value(&text).unwrap(),
        json!({"name": "a.txt", "text": "one\n"})
    );
    assert_eq!(
        serde_json::to_value(&binary).unwrap(),
        json!({"name": "a.png", "base64": "iVA="})
    );
}

#[test]
fn a_flattened_bytes_field_adds_text_and_base64_to_the_properties_of_the_row_schema() {
    let schema = schemars::schema_for!(Flat);
    let properties = schema.get("properties").unwrap();

    assert_eq!(properties["text"]["type"], json!("string"));
    assert_eq!(properties["base64"]["contentEncoding"], json!("base64"));
    assert_eq!(
        schema.get("oneOf"),
        Some(&json!([{"required": ["text"]}, {"required": ["base64"]}]))
    );
    assert_eq!(
        schema.get("description"),
        Some(&json!(
            "A file whose bytes sit at the top level of the row, as `text` or\n`base64`."
        ))
    );
}

#[test]
fn a_flattened_optional_bytes_field_adds_text_and_base64_to_the_properties_and_may_be_absent() {
    let schema = schemars::schema_for!(MaybeFlat);
    let properties = schema.get("properties").unwrap();
    let row = MaybeFlat {
        name: "a.json",
        content: None,
    };

    assert_eq!(properties["text"]["type"], json!("string"));
    assert_eq!(properties["base64"]["type"], json!("string"));
    assert_eq!(
        serde_json::to_value(&row).unwrap(),
        json!({"name": "a.json"})
    );
}

#[test]
fn a_flattened_bytes_field_stays_an_object_while_the_text_mode_is_set() {
    let row = Flat {
        name: "a.txt",
        content: Bytes::from("one\n".to_owned()),
    };

    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(&row).unwrap()
    };

    assert_eq!(value, json!({"name": "a.txt", "text": "one\n"}));
}

#[test]
fn bytes_display_as_text_when_they_are_utf8_and_as_a_count_otherwise() {
    assert_eq!(Bytes::from("North\n".to_owned()).to_string(), "North\n");
    assert_eq!(Bytes::new(vec![0x89, 0x50, 0xff]).to_string(), "<3 bytes>");
}
