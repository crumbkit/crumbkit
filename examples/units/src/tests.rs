use crumbkit::schemars::schema_for;
use crumbkit::{Field, FieldValue};

use super::State;

#[test]
fn a_value_type_from_a_crate_without_serde_gives_its_json_form_to_a_failure_field() {
    let field = Field::shown("state", &State::Closed);

    assert_eq!(field.value(), &FieldValue::from("closed"));
}

#[test]
fn a_value_type_from_a_crate_without_schemars_has_a_string_schema() {
    let schema = schema_for!(State);

    assert_eq!(
        schema.get("type").and_then(|kind| kind.as_str()),
        Some("string")
    );
}
