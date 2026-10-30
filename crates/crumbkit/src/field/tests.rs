//! Tests for the named values of a failure.

use std::borrow::Cow;
use std::fmt;

use serde::Serializer;

use super::{Field, FieldValue};
use crate::shown::TextCells;
use crate::{Duration, Shown};

#[test]
fn a_field_keeps_its_name_and_value() {
    let field = Field::new("book", 7_u32);

    assert_eq!(field.name(), "book");
    assert_eq!(field.value(), &FieldValue::Unsigned(7));
}

#[test]
fn a_text_value_borrows_a_str_and_owns_a_string() {
    let title = "Frankenstein".to_owned();

    assert_eq!(
        Field::new("title", title.as_str()).value(),
        &FieldValue::Text(Cow::Borrowed("Frankenstein"))
    );
    assert_eq!(
        Field::new("title", &title).value(),
        &FieldValue::Text(Cow::Borrowed("Frankenstein"))
    );
    assert_eq!(
        Field::new("title", title.clone()).value(),
        &FieldValue::Text(Cow::Owned(title))
    );
}

#[test]
fn signed_integers_become_integer_values_and_unsigned_ones_unsigned_values() {
    assert_eq!(FieldValue::from(-3_i8), FieldValue::Integer(-3));
    assert_eq!(FieldValue::from(-3_i16), FieldValue::Integer(-3));
    assert_eq!(FieldValue::from(-3_i32), FieldValue::Integer(-3));
    assert_eq!(FieldValue::from(i64::MIN), FieldValue::Integer(i64::MIN));
    assert_eq!(FieldValue::from(3_u8), FieldValue::Unsigned(3));
    assert_eq!(FieldValue::from(3_u16), FieldValue::Unsigned(3));
    assert_eq!(FieldValue::from(3_u32), FieldValue::Unsigned(3));
    assert_eq!(FieldValue::from(u64::MAX), FieldValue::Unsigned(u64::MAX));
    assert_eq!(FieldValue::from(3_usize), FieldValue::Unsigned(3));
    assert_eq!(FieldValue::from(true), FieldValue::Boolean(true));
}

#[test]
fn each_value_serializes_as_its_json_value() {
    let values = [
        FieldValue::from("seven"),
        FieldValue::from(-7_i64),
        FieldValue::from(7_u64),
        FieldValue::from(false),
        FieldValue::Null,
        FieldValue::list([FieldValue::from(1_u8), FieldValue::list(["a", "b"])]),
        FieldValue::object([
            ("zeta", FieldValue::from(1_u8)),
            ("alpha", FieldValue::Null),
        ]),
    ];

    assert_eq!(
        serde_json::to_string(&values).unwrap(),
        r#"["seven",-7,7,false,null,[1,["a","b"]],{"zeta":1,"alpha":null}]"#
    );
}

#[test]
fn a_missing_option_becomes_null_and_a_present_one_becomes_its_value() {
    assert_eq!(FieldValue::from(None::<u8>), FieldValue::Null);
    assert_eq!(FieldValue::from(Some("x")), FieldValue::from("x"));
}

#[test]
fn an_owned_value_keeps_the_borrowed_value_it_came_from() {
    let title = "Frankenstein".to_owned();
    let key = "title".to_owned();
    let borrowed = FieldValue::list([FieldValue::object([(
        key.as_str(),
        FieldValue::from(title.as_str()),
    )])]);

    let owned: FieldValue<'static> = borrowed.clone().into_owned();

    assert_eq!(owned, borrowed);
}

#[test]
fn a_name_of_lowercase_letters_digits_and_underscores_follows_the_rule() {
    assert!(Field::new("book_id_2", 1_u8).follows_name_rule());
}

#[test]
fn an_empty_name_or_one_with_another_character_breaks_the_rule() {
    for name in ["", "Book", "book-id", "book id", "bookId", "livré"] {
        assert!(!Field::new(name, 1_u8).follows_name_rule(), "{name:?}");
    }
}

#[test]
fn from_json_reads_each_json_value_as_the_field_value_that_serializes_to_it() {
    let json = serde_json::json!({
        "title": "Frankenstein",
        "count": 7,
        "offset": -7,
        "kept": true,
        "found": null,
        "tags": ["a", ["b"]],
        "lines": {"a": 1},
    });

    let value = FieldValue::from_json(&json);

    assert_eq!(
        value.map(|value| serde_json::to_value(value).unwrap()),
        Some(json)
    );
}

#[test]
fn from_json_reads_a_non_negative_integer_as_unsigned_a_negative_one_as_signed_and_borrows_text() {
    let read = |json: serde_json::Value| FieldValue::from_json(&json).map(FieldValue::into_owned);

    assert_eq!(read(serde_json::json!(7)), Some(FieldValue::Unsigned(7)));
    assert_eq!(
        read(serde_json::json!(u64::MAX)),
        Some(FieldValue::Unsigned(u64::MAX))
    );
    assert_eq!(read(serde_json::json!(-7)), Some(FieldValue::Integer(-7)));
    assert_eq!(
        read(serde_json::json!(i64::MIN)),
        Some(FieldValue::Integer(i64::MIN))
    );
    let text = serde_json::json!("seven");
    assert!(matches!(
        FieldValue::from_json(&text),
        Some(FieldValue::Text(Cow::Borrowed("seven")))
    ));
}

#[test]
fn from_json_returns_none_for_a_fraction_or_an_integer_beyond_u64_at_any_depth() {
    for json in [
        "1.5",
        "1e3",
        "18446744073709551616",
        "-9223372036854775809",
        "[1, 2.5]",
        r#"{"a": {"b": 0.1}}"#,
    ] {
        let value: serde_json::Value = serde_json::from_str(json).unwrap();

        assert_eq!(FieldValue::from_json(&value), None, "{json}");
    }
}

/// A weight in grams, which reads `1.5 kg` in text and `1500` in JSON.
struct Grams(u64);

impl fmt::Display for Grams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{} kg", self.0 / 1_000, self.0 % 1_000 / 100)
    }
}

impl Shown for Grams {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

/// A ratio, whose JSON form is a fraction that a field value cannot hold.
struct Ratio(f64);

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0 * 100.0)
    }
}

impl Shown for Ratio {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0)
    }
}

/// A place on a shelf, whose JSON form is an object.
struct Spot;

impl fmt::Display for Spot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("shelf 2, slot 7")
    }
}

impl Shown for Spot {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;

        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("shelf", &2)?;
        map.serialize_entry("slot", &7)?;
        map.end()
    }
}

#[test]
fn a_shown_field_holds_the_json_form_of_the_value() {
    let field = Field::shown("weight", &Grams(1_500));

    assert_eq!(field.name(), "weight");
    assert_eq!(field.value(), &FieldValue::Unsigned(1_500));
    assert_eq!(Grams(1_500).to_string(), "1.5 kg");
}

#[test]
fn a_shown_field_holds_the_text_of_a_value_with_the_default_json_form() {
    let waited = Duration::new(std::time::Duration::from_secs(90));

    assert_eq!(
        Field::shown("waited", &waited).value(),
        &FieldValue::from("PT1M30S")
    );
}

#[test]
fn a_shown_field_holds_an_object_json_form_as_an_object() {
    assert_eq!(
        Field::shown("spot", &Spot).value(),
        &FieldValue::object([
            ("shelf", FieldValue::from(2_u32)),
            ("slot", FieldValue::from(7_u32)),
        ])
    );
}

#[test]
fn a_shown_field_whose_json_form_has_a_fraction_holds_the_text_form() {
    assert_eq!(
        Field::shown("ratio", &Ratio(0.5)).value(),
        &FieldValue::from("50%")
    );
}

#[test]
fn a_shown_field_holds_the_json_form_while_the_text_mode_is_set() {
    let field = {
        let _cells = TextCells::enter();
        Field::shown("weight", &Grams(1_500))
    };

    assert_eq!(field.value(), &FieldValue::Unsigned(1_500));
}
