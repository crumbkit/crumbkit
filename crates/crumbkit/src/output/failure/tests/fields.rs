//! Tests for how `finish_to` prints a failure's named values.

use clap::{CommandFactory as _, Parser as _};

use super::{Missing, Tool, fail_output, fail_with, missing, run_list, two_causes};
use crate::FieldValue;
use crate::help::Guide;

/// Returns a failure with `fields`.
fn with_fields(fields: Vec<(&'static str, FieldValue<'static>)>) -> Missing {
    Missing {
        fields,
        ..missing(run_list())
    }
}

#[test]
fn json_holds_the_fields_after_the_cause_as_one_object_in_their_order() {
    let failure = Missing {
        cause: Some(two_causes()),
        ..with_fields(vec![
            ("planet", "Vulcan".into()),
            ("moons", 0_u8.into()),
            ("offset", (-3_i32).into()),
            ("named", true.into()),
        ])
    };

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","cause":"the catalogue cannot be read: permission denied","fields":{"planet":"Vulcan","moons":0,"offset":-3,"named":true},"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn json_holds_a_list_an_object_and_null_as_json_values() {
    let failure = with_fields(vec![
        ("moons", FieldValue::list(["Phobos", "Deimos"])),
        (
            "orbit",
            FieldValue::object([
                ("days", FieldValue::from(687_u16)),
                ("known", FieldValue::Null),
            ]),
        ),
        ("found", FieldValue::Null),
    ]);

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","fields":{"moons":["Phobos","Deimos"],"orbit":{"days":687,"known":null},"found":null},"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
}

#[test]
fn text_output_prints_no_line_for_the_fields() {
    let finished = fail_with(&[], with_fields(vec![("planet", "Vulcan".into())]));

    assert_eq!(
        finished.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list`\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

// A release build leaves the field out without the warning.
#[cfg(debug_assertions)]
#[test]
fn a_field_whose_name_breaks_the_rule_is_left_out_with_a_warning_in_a_debug_build() {
    let failure = with_fields(vec![("Planet", "Vulcan".into()), ("moons", 0_u8.into())]);

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","fields":{"moons":0},"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
    assert_eq!(
        finished.err,
        "warning: crumbkit left out the field \"Planet\" of the failure solar::planets::not_found, because a field name is one or more lowercase ASCII letters, digits and underscores, and Guide::field_names does not list it\n"
    );
}

#[test]
fn a_field_whose_name_the_guide_lists_prints_though_it_breaks_the_rule() {
    const LISTING: Guide = Guide::new().field_names(&["planetId", "Content-Type"]);
    let view = Tool::try_parse_from(["tool", "list", "--json"])
        .unwrap()
        .view;
    let mut output =
        crate::Output::new(view, Tool::command(), &LISTING, ["tool", "list", "--json"]);
    let failure = with_fields(vec![
        ("planetId", "Vulcan".into()),
        ("Content-Type", "text/plain".into()),
        ("Moons", 0_u8.into()),
    ]);

    let finished = fail_output(&mut output, crate::Error::from(failure));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","fields":{"planetId":"Vulcan","Content-Type":"text/plain"},"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
}

// A release build leaves the field out without the warning.
#[cfg(debug_assertions)]
#[test]
fn a_field_whose_name_an_earlier_field_has_is_left_out_with_a_warning_in_a_debug_build() {
    let failure = with_fields(vec![
        ("planet", "Vulcan".into()),
        ("planet", "Krypton".into()),
    ]);

    let finished = fail_with(&[], failure);

    assert_eq!(
        finished.err,
        concat!(
            "warning: crumbkit left out the field \"planet\" of the failure solar::planets::not_found, because an earlier field has that name\n",
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list`\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

// A release build leaves the entry out without the warning.
#[cfg(debug_assertions)]
#[test]
fn a_key_that_repeats_an_earlier_key_of_its_object_is_left_out_with_a_warning_in_a_debug_build() {
    let orbit = FieldValue::object([("days", 687_u16), ("moons", 2), ("days", 1)]);
    let failure = with_fields(vec![("orbit", FieldValue::list([orbit]))]);

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","fields":{"orbit":[{"days":687,"moons":2}]},"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
    assert_eq!(
        finished.err,
        "warning: crumbkit left out the key \"days\" of an object in the field \"orbit\" of the failure solar::planets::not_found, because an earlier key of that object has that name\n"
    );
}
