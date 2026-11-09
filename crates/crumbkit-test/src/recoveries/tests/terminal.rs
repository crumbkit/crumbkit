//! Tests for how the recovery check treats a step that a person must run in
//! a terminal, which it follows as it follows a `run` step.

use super::{CODES, OTHERS, SHOW_7, Setup, failure, problems, recoveries, run};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::Recoveries;

/// Returns a step that a person runs in a terminal, in JSON.
fn terminal(call: &str) -> String {
    format!(r#"{{"kind":"terminal","call":"{call}"}}"#)
}

/// Returns the tool whose `books show 7` fails with `tool::books::not_found`
/// and offers `tool books list` in a terminal, and whose `books list` exits
/// with `code` and prints `stdout`.
fn tool(label: &str, code: u8, stdout: &'static str) -> Setup {
    let show = failure("tool::books::not_found", &terminal("tool books list"));
    Setup::new(
        label,
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", code, stdout, ""),
        ],
    )
}

#[test]
fn a_terminal_step_that_exits_zero_passes_and_the_report_shows_it_run() {
    let setup = tool("recoveries-terminal-exits-0", 0, "{}\n");

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
    assert!(
        setup
            .report()
            .contains("  run tool books list\n    exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_terminal_step_that_the_parser_refuses_fails_the_check_as_a_run_step_does() {
    let setup = tool("recoveries-terminal-refused", 2, "");

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(message.contains("the step `tool books list`"), "{message}");
}

#[test]
fn a_terminal_step_that_names_another_tool_fails_the_check_as_a_run_step_does() {
    let show = failure("tool::books::not_found", &terminal("other books list"));
    let setup = Setup::new(
        "recoveries-terminal-other-tool",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(
        message.contains("must start with the tool's name"),
        "{message}"
    );
}

#[test]
fn a_terminal_step_counts_as_a_next_step_of_a_declared_code() {
    let setup = tool("recoveries-terminal-next-step", 0, "{}\n");

    let checked = Recoveries::new(Examples::new(&setup.script, ""), CODES)
        .starts(SHOW_7)
        .unprovoked(OTHERS);

    assert_eq!(problems(&setup, &checked), None);
}

#[test]
fn a_terminal_step_with_a_placeholder_fails_the_check_even_when_a_field_could_fill_it() {
    let show = format!(
        r#"{{"schema":"tool/failure/v1","code":"tool::books::not_found","message":"m","fields":{{"id":"7"}},"recovery":{}}}"#,
        terminal("tool books list <id>")
    );
    let setup = Setup::new(
        "recoveries-terminal-placeholder",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(
        message.contains("the terminal step `tool books list <id>` has the placeholder <id>"),
        "{message}"
    );
}

#[test]
fn a_run_step_with_the_same_placeholder_is_filled_from_the_field() {
    let show = format!(
        r#"{{"schema":"tool/failure/v1","code":"tool::books::not_found","message":"m","fields":{{"id":"7"}},"recovery":{}}}"#,
        run("tool books list <id>")
    );
    let setup = Setup::new(
        "recoveries-run-placeholder",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list 7 --json", 0, "{}\n", ""),
        ],
    );
    let allow = [Allow::new(
        Rule::FieldValue,
        "books show",
        "The test only shows that a run step is filled.",
    )];

    let checked = Recoveries::new(Examples::new(&setup.script, "").allow(&allow), CODES)
        .starts(SHOW_7)
        .unprovoked(OTHERS);

    assert_eq!(problems(&setup, &checked), None);
}

#[test]
fn the_warning_for_a_terminal_call_with_a_placeholder_fails_the_check() {
    let show = failure(
        "tool::books::not_found",
        r#"{"kind":"none","reason":"the suggested command is not available"}"#,
    );
    let setup = Setup::new(
        "recoveries-terminal-warning",
        &[(
            "books show 7 --json",
            1,
            show.as_str(),
            "warning: tool left out a recovery of the failure tool::books::not_found, because the terminal call holds a placeholder\n",
        )],
    );

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(message.contains("warning"), "{message}");
}
