//! Tests for how `finish_to` prints a recovery that a person must run in a
//! terminal.

use std::process::ExitCode;

use super::{Missing, fail_with, missing, run_list};
use crate::{Call, Recovery};

/// Returns the recovery that names `tool list`, in a terminal.
fn terminal_list() -> Recovery {
    Recovery::Terminal(Call::new(&["list"]))
}

#[test]
fn text_prints_a_terminal_recovery_as_a_help_line_naming_the_command_and_a_terminal() {
    let finished = fail_with(&[], missing(terminal_list()));

    assert_eq!(
        finished.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list` in a terminal\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

#[test]
fn json_holds_a_terminal_recovery_tagged_by_kind_with_its_rendered_call() {
    let finished = fail_with(&["--json"], missing(terminal_list()));

    assert!(
        finished
            .out
            .contains(r#""recovery":{"kind":"terminal","call":"tool list"}"#),
        "{}",
        finished.out
    );
}

#[test]
fn a_terminal_call_that_does_not_resolve_prints_as_a_run_call_that_does_not_resolve() {
    let unresolved = || missing(Recovery::Terminal(Call::new(&["remove"])));

    let text = fail_with(&[], unresolved());
    let json = fail_with(&["--json"], unresolved());

    assert_eq!(text.code, ExitCode::FAILURE);
    assert!(
        text.err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        text.err
    );
    assert!(
        json.out.contains(
            r#""recovery":{"kind":"none","reason":"the suggested command is not available"}"#
        ),
        "{}",
        json.out
    );
}

#[cfg(debug_assertions)]
#[test]
fn a_terminal_call_that_does_not_resolve_adds_a_warning_line_in_a_debug_build() {
    let finished = fail_with(&[], missing(Recovery::Terminal(Call::new(&["remove"]))));

    assert!(finished.err.starts_with("warning: "), "{}", finished.err);
    assert!(finished.err.contains("remove"), "{}", finished.err);
}

#[test]
fn a_terminal_alternative_prints_after_the_recovery() {
    let failure = Missing {
        alternatives: vec![terminal_list()],
        ..missing(run_list())
    };

    let text = fail_with(&[], failure);

    assert_eq!(
        text.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list`\n",
            "  help:  run `tool list` in a terminal\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

#[test]
fn a_terminal_alternative_that_does_not_resolve_is_left_out() {
    let failure = Missing {
        alternatives: vec![Recovery::Terminal(Call::new(&["remove"]))],
        ..missing(run_list())
    };

    let json = fail_with(&["--json"], failure);

    assert!(!json.out.contains("alternatives"), "{}", json.out);
}

#[test]
fn a_terminal_call_with_a_value_prints_the_value_quoted_for_a_shell() {
    let call = Recovery::Terminal(Call::typed(vec!["list".to_owned(), "to read".to_owned()]));

    let finished = fail_with(&[], missing(call));

    assert!(
        finished
            .err
            .contains("\n  help:  run `tool list 'to read'` in a terminal\n"),
        "{}",
        finished.err
    );
}

/// Returns the calls that hold a placeholder: one from `Call::arg`, one from
/// `Call::all` and one from `Call::typed_all`.
fn calls_with_a_placeholder() -> [Call; 3] {
    [
        Call::new(&["show"]).arg("ids"),
        Call::new(&["show"]).all("ids", "left_out"),
        Call::typed_all(vec!["show".to_owned()], 1, "<ID>...".to_owned(), "left_out"),
    ]
}

#[test]
fn a_terminal_call_with_a_placeholder_prints_as_a_call_that_does_not_resolve() {
    for call in calls_with_a_placeholder() {
        let text = fail_with(&[], missing(Recovery::Terminal(call.clone())));
        let json = fail_with(&["--json"], missing(Recovery::Terminal(call)));

        assert!(
            text.err
                .contains("\n  help:  no next step: the suggested command is not available\n"),
            "{}",
            text.err
        );
        assert!(
            json.out.contains(
                r#""recovery":{"kind":"none","reason":"the suggested command is not available"}"#
            ),
            "{}",
            json.out
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_terminal_call_with_a_placeholder_adds_a_warning_line_in_a_debug_build() {
    for call in calls_with_a_placeholder() {
        let finished = fail_with(&[], missing(Recovery::Terminal(call)));

        assert!(finished.err.starts_with("warning: "), "{}", finished.err);
        assert!(finished.err.contains("placeholder"), "{}", finished.err);
    }
}

#[test]
fn a_terminal_alternative_with_a_placeholder_is_left_out() {
    for call in calls_with_a_placeholder() {
        let failure = Missing {
            alternatives: vec![Recovery::Terminal(call)],
            ..missing(run_list())
        };

        let json = fail_with(&["--json"], failure);

        assert!(!json.out.contains("alternatives"), "{}", json.out);
    }
}

#[test]
fn a_terminal_call_with_only_concrete_values_prints_as_a_terminal_step() {
    let call = Call::new(&["show"]).value("ids", "7");

    let json = fail_with(&["--json"], missing(Recovery::Terminal(call)));

    assert!(
        json.out
            .contains(r#""recovery":{"kind":"terminal","call":"tool show 7"}"#),
        "{}",
        json.out
    );
}
