//! Tests for shown values: the two forms, the schema that `shown!` writes,
//! and the text mode that picks a form. crumbkit's tests check how
//! `finish_to`, a stream and a failure print them.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, schema_for};
use serde::{Serialize, Serializer};
use serde_json::json;

use super::{JsonForm, Shown, TextCells};

/// A shelf's state, which reads the same in text and JSON.
#[derive(Clone, Copy)]
enum State {
    /// The shelf is open.
    Open,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => f.write_str("open"),
        }
    }
}

impl Shown for State {}

crate::shown!(State);

/// A number of pages, which reads `432 pages` in text and `432` in JSON.
#[derive(Clone, Copy)]
struct Pages(u32);

impl fmt::Display for Pages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} pages", self.0)
    }
}

impl Shown for Pages {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        u32::json_schema(generator)
    }
}

crate::shown!(Pages);

/// A shelf's place in the library, which a schema defines under a name.
struct Place(&'static str);

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Shown for Place {
    fn schema_name() -> Option<Cow<'static, str>> {
        Some("Place".into())
    }
}

crate::shown!(Place);

/// A row of shown values.
#[derive(Serialize, JsonSchema)]
struct Shelf {
    /// The shelf's state.
    state: State,
    /// The pages on the shelf.
    pages: Pages,
    /// The pages that were lent out, if any were.
    lent: Option<Pages>,
    /// Where the shelf stands.
    place: Place,
}

fn shelf() -> Shelf {
    Shelf {
        state: State::Open,
        pages: Pages(432),
        lent: None,
        place: Place("east wing"),
    }
}

#[test]
fn a_value_with_the_default_json_form_serializes_as_its_text() {
    assert_eq!(serde_json::to_value(State::Open).unwrap(), json!("open"));
}

#[test]
fn a_value_with_its_own_json_form_serializes_in_that_form_outside_text_mode() {
    assert_eq!(serde_json::to_value(Pages(432)).unwrap(), json!(432));
}

#[test]
fn a_value_serializes_as_its_display_while_the_text_mode_is_set() {
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(shelf()).unwrap()
    };

    assert_eq!(
        value,
        json!({"state": "open", "pages": "432 pages", "lent": null, "place": "east wing"})
    );
}

#[test]
fn a_row_of_shown_values_serializes_in_their_json_forms_outside_text_mode() {
    assert_eq!(
        serde_json::to_value(shelf()).unwrap(),
        json!({"state": "open", "pages": 432, "lent": null, "place": "east wing"})
    );
}

#[test]
fn the_json_form_ignores_the_text_mode() {
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(JsonForm(&Pages(7))).unwrap()
    };

    assert_eq!(value, json!(7));
}

#[test]
fn the_default_schema_is_an_inline_string() {
    let schema = schema_for!(Shelf).to_value();

    assert_eq!(
        schema.pointer("/properties/state"),
        Some(&json!({"description": "The shelf's state.", "type": "string"}))
    );
}

#[test]
fn a_schema_of_its_own_is_inline_with_the_fields_description() {
    let schema = schema_for!(Shelf).to_value();

    assert_eq!(
        schema.pointer("/properties/pages"),
        Some(&json!({
            "description": "The pages on the shelf.",
            "type": "integer",
            "format": "uint32",
            "minimum": 0,
        }))
    );
    assert_eq!(
        schema.pointer("/properties/lent/type"),
        Some(&json!(["integer", "null"]))
    );
}

#[test]
fn a_schema_name_defines_the_schema_under_that_name() {
    let schema = schema_for!(Shelf).to_value();

    assert_eq!(
        schema.pointer("/properties/place/$ref"),
        Some(&json!("#/$defs/Place"))
    );
    assert_eq!(
        schema.pointer("/$defs/Place"),
        Some(&json!({"type": "string"}))
    );
}

#[test]
fn the_schema_id_names_the_module_and_the_type() {
    assert_eq!(Pages::schema_id(), "crumbkit::shown::tests::Pages");
}

#[test]
fn the_text_mode_ends_when_its_guard_is_dropped() {
    drop(TextCells::enter());

    assert!(!TextCells::active());
    assert_eq!(serde_json::to_value(Pages(1)).unwrap(), json!(1));
}

#[test]
fn an_inner_guard_restores_the_text_mode_that_it_found() {
    let outer = TextCells::enter();

    drop(TextCells::enter());
    let value = serde_json::to_value(Pages(1)).unwrap();
    drop(outer);

    assert_eq!(value, json!("1 pages"));
    assert!(!TextCells::active());
}

#[test]
fn the_text_mode_of_one_thread_leaves_another_thread_in_the_json_form() {
    let _cells = TextCells::enter();

    let other = std::thread::spawn(|| serde_json::to_value(Pages(3)).unwrap())
        .join()
        .unwrap();

    assert_eq!(other, json!(3));
}
