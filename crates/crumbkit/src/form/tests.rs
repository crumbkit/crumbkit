//! Tests for the failure that crumbkit-test reads back.

use super::ReadFailure;
use crate::FieldValue;

#[test]
fn a_read_failure_keeps_the_fields_it_is_given_with_their_values_and_has_none_without_them() {
    let fields = vec![
        ("id".to_owned(), FieldValue::Unsigned(7)),
        ("found".to_owned(), FieldValue::Null),
        ("tags".to_owned(), FieldValue::list(["red".to_owned()])),
    ];

    let read = ReadFailure::new("solar.planets.not_found".to_owned(), Vec::new())
        .with_fields(fields.clone());

    assert_eq!(read.fields(), fields);
    assert_eq!(
        ReadFailure::new("solar.planets.not_found".to_owned(), Vec::new()).fields(),
        []
    );
}
