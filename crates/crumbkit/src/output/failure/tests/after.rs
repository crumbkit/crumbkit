//! Tests for how `finish_to` prints a recovery that puts a repair before
//! its next step.

use super::{fail_with, missing};
use crate::{Call, Recovery, Then};

#[test]
fn a_repair_before_a_retry_prints_the_repair_then_run_the_same_command_again() {
    let finished = fail_with(
        &[],
        missing(Recovery::after("move the file away", Then::Retry)),
    );

    assert_eq!(
        finished.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  move the file away, then run the same command again\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

#[test]
fn a_repair_before_a_retry_prints_as_an_after_step_with_a_retry_then_in_json() {
    let finished = fail_with(
        &["--json"],
        missing(Recovery::after("move the file away", Then::Retry)),
    );

    assert!(
        finished.out.contains(
            r#""recovery":{"kind":"after","repair":"move the file away","then":{"kind":"retry"}}"#
        ),
        "{}",
        finished.out
    );
}

#[test]
fn a_repair_before_a_run_renders_the_call_on_the_help_line_and_in_then() {
    let call = || Recovery::after("move the file away", Then::Run(Call::new(&["list"])));

    let text = fail_with(&[], missing(call()));
    let json = fail_with(&["--json"], missing(call()));

    assert!(
        text.err
            .contains("\n  help:  move the file away, then run `tool list`\n"),
        "{}",
        text.err
    );
    assert!(
        json.out.contains(
            r#""recovery":{"kind":"after","repair":"move the file away","then":{"kind":"run","call":"tool list"}}"#
        ),
        "{}",
        json.out
    );
}

// A release build prints the same failure without the warning.
#[cfg(debug_assertions)]
#[test]
fn a_repair_before_a_call_that_does_not_render_prints_no_next_step_and_warns_in_a_debug_build() {
    let finished = fail_with(
        &[],
        missing(Recovery::after(
            "move the file away",
            Then::Run(Call::new(&["remove"])),
        )),
    );

    assert!(
        finished
            .err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        finished.err
    );
    assert!(finished.err.starts_with("warning: "), "{}", finished.err);
}

#[test]
fn a_repair_before_a_terminal_call_with_a_placeholder_prints_no_next_step() {
    let call = Call::new(&["show"]).arg("ids");

    let text = fail_with(
        &[],
        missing(Recovery::after("move the file away", Then::Terminal(call))),
    );

    assert!(
        text.err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        text.err
    );
}

#[test]
fn the_failure_schema_admits_an_after_step() {
    let schema = super::super::document_schema().to_value();
    let steps = schema["properties"]["recovery"]["oneOf"]
        .as_array()
        .unwrap();

    let after = steps
        .iter()
        .find(|step| step["properties"]["kind"]["const"] == "after")
        .expect("the step schema holds an after member");
    assert_eq!(
        after["required"],
        serde_json::json!(["kind", "repair", "then"])
    );
    assert_eq!(after["properties"]["repair"]["type"], "string");
    let then = after["properties"]["then"]["oneOf"].as_array().unwrap();
    let kinds: Vec<_> = then
        .iter()
        .map(|step| step["properties"]["kind"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["run", "terminal", "retry"]);
    let alternatives = schema["properties"]["alternatives"]["items"]["oneOf"]
        .as_array()
        .unwrap();
    assert!(alternatives.contains(after));
}

#[test]
fn the_failure_schema_rejects_an_after_step_whose_then_is_a_hint() {
    let schema = super::super::document_schema().to_value();
    let steps = schema["properties"]["recovery"]["oneOf"]
        .as_array()
        .unwrap();

    let after = steps
        .iter()
        .find(|step| step["properties"]["kind"]["const"] == "after")
        .unwrap();

    let kinds = after["properties"]["then"]["oneOf"].to_string();
    assert!(!kinds.contains(r#""hint""#), "{kinds}");
    assert!(!kinds.contains(r#""none""#), "{kinds}");
    assert!(!kinds.contains(r#""after""#), "{kinds}");
}

#[test]
fn a_repair_before_a_terminal_call_prints_the_call_in_a_terminal() {
    let finished = fail_with(
        &[],
        missing(Recovery::after(
            "move the file away",
            Then::Terminal(Call::new(&["list"])),
        )),
    );

    assert!(
        finished
            .err
            .contains("\n  help:  move the file away, then run `tool list` in a terminal\n"),
        "{}",
        finished.err
    );
}
