//! Tests for the check that a JSON value matches a schema.

use serde_json::json;

use super::conforms;

/// Returns a schema of an object with a required integer `id`, an
/// optional `title` that a definition describes, and no other fields.
fn book() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "integer", "minimum": 0 },
            "title": { "$ref": "#/$defs/Title" },
        },
        "required": ["id"],
        "additionalProperties": false,
        "$defs": { "Title": { "type": ["string", "null"] } },
    })
}

#[test]
fn a_value_with_every_required_field_and_declared_types_conforms() {
    assert_eq!(
        conforms(&book(), &json!({"id": 1, "title": "Emma"})),
        Ok(())
    );
    assert_eq!(conforms(&book(), &json!({"id": 1, "title": null})), Ok(()));
}

#[test]
fn a_value_without_a_required_field_does_not_conform() {
    assert!(conforms(&book(), &json!({"title": "Emma"})).is_err());
}

#[test]
fn a_field_that_the_schema_does_not_declare_fails_when_additional_properties_is_false() {
    assert!(conforms(&book(), &json!({"id": 1, "isbn": null})).is_err());
}

#[test]
fn a_field_of_the_wrong_type_through_a_reference_does_not_conform() {
    assert!(conforms(&book(), &json!({"id": 1, "title": 7})).is_err());
    assert!(conforms(&book(), &json!({"id": -1})).is_err());
}

#[test]
fn one_of_needs_exactly_one_member_to_match() {
    let schema = json!({"oneOf": [{"const": "a"}, {"type": "string"}]});

    assert_eq!(conforms(&schema, &json!("b")), Ok(()));
    assert!(conforms(&schema, &json!("a")).is_err());
    assert!(conforms(&schema, &json!(1)).is_err());
}

#[test]
fn a_keyword_that_the_check_does_not_handle_fails_the_check() {
    let schema = json!({"type": "object", "unevaluatedProperties": false});

    assert!(conforms(&schema, &json!({})).is_err());
}
