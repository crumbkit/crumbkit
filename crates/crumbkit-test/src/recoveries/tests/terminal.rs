//! Tests for how the recovery check treats a step that a person must run in
//! a terminal, which it follows as it follows a `run` step.

use super::{CODES, OTHERS, SHOW_7, Setup, failure, problems, recoveries};
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
