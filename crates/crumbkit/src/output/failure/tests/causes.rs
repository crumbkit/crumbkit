//! Tests for how `finish_to` prints the causes that a failure lists.

use super::{Missing, fail_with, missing, run_list, two_causes};
use crate::Failure;

/// Returns a failure that lists `listed` as its causes.
fn with_causes(listed: Vec<&'static str>) -> Missing {
    Missing {
        listed,
        ..missing(run_list())
    }
}

#[test]
fn the_default_causes_of_a_failure_are_empty() {
    assert_eq!(missing(run_list()).causes(), Vec::<String>::new());
}

#[test]
fn each_listed_cause_prints_as_a_cause_line_after_the_source_chain() {
    let failure = Missing {
        cause: Some(two_causes()),
        ..with_causes(vec![
            "the flow intake has no start step",
            "the flow billing names an unknown step",
        ])
    };

    let finished = fail_with(&[], failure);

    let expected = "\
error: the solar system has no planet Vulcan
  cause: the catalogue cannot be read
  cause: permission denied
  cause: the flow intake has no start step
  cause: the flow billing names an unknown step
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(finished.err, expected);
}

#[test]
fn a_failure_without_listed_causes_prints_the_same_text_and_document_as_before() {
    let text = fail_with(&[], missing(run_list()));
    let json = fail_with(&["--json"], missing(run_list()));

    assert_eq!(
        text.err,
        "error: the solar system has no planet Vulcan\n  help:  run `tool list`\n  code:  solar::planets::not_found\n"
    );
    assert!(!json.out.contains("causes"), "{}", json.out);
}

#[test]
fn json_holds_the_listed_causes_in_order_and_leaves_cause_to_the_source_chain() {
    let failure = Missing {
        cause: Some(two_causes()),
        ..with_causes(vec!["first reason", "second reason"])
    };

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"tool/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","cause":"the catalogue cannot be read: permission denied","causes":["first reason","second reason"],"recovery":{"kind":"run","call":"tool list"}}"#,
            "\n"
        )
    );
}

#[test]
fn json_holds_causes_before_fields() {
    let failure = Missing {
        fields: vec![("planet", "Vulcan".into())],
        ..with_causes(vec!["a reason"])
    };

    let finished = fail_with(&["--json"], failure);

    let causes = finished.out.find("\"causes\"").unwrap();
    let fields = finished.out.find("\"fields\"").unwrap();
    assert!(causes < fields, "{}", finished.out);
}

#[test]
fn a_listed_cause_with_a_line_break_prints_as_one_escaped_line() {
    let finished = fail_with(&[], with_causes(vec!["first line\nsecond line"]));

    assert!(
        finished.err.contains("  cause: first line\\nsecond line\n"),
        "{}",
        finished.err
    );
}
