//! Tests for the named values of a failure.

use std::borrow::Cow;

use super::{Field, FieldValue};

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
