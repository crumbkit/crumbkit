use std::env::VarError;
use std::ffi::OsString;

use super::{Caller, Mark, identify, marked};
use crate::program::Program;
use crumbkit::ToolName;

/// Returns the agent that is-ai-agent detects in an environment that holds
/// only `pairs`, with no marker files.
fn agent(pairs: &'static [(&'static str, &'static str)]) -> Option<is_ai_agent::Agent> {
    is_ai_agent::detect_with(env(pairs), |_| false)
}

/// Returns an environment that holds only `pairs`.
fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

#[test]
fn an_agent_with_a_session_id_is_the_caller_and_its_session_id_is_the_session() {
    let caller = identify(
        agent(&[("CLAUDECODE", "1"), ("CLAUDE_CODE_SESSION_ID", "s-1")]),
        true,
        || "4242".to_owned(),
    );

    assert_eq!(
        caller,
        Caller {
            name: "claude-code",
            session: "s-1".to_owned(),
        }
    );
}

#[test]
fn an_agent_without_a_session_id_has_the_platform_s_fallback_as_its_session() {
    let caller = identify(agent(&[("CLAUDECODE", "1")]), false, || "4242".to_owned());

    assert_eq!(
        caller,
        Caller {
            name: "claude-code",
            session: "4242".to_owned(),
        }
    );
}

#[test]
fn without_an_agent_a_terminal_on_standard_input_means_a_person() {
    let caller = identify(agent(&[]), true, || "4242".to_owned());

    assert_eq!(
        caller,
        Caller {
            name: "person",
            session: "4242".to_owned(),
        }
    );
}

#[test]
fn without_an_agent_or_a_terminal_the_caller_is_unknown() {
    let caller = identify(agent(&[]), false, || "4242".to_owned());

    assert_eq!(caller.name, "unknown");
}

/// Returns the mark of a run of shelf in an environment whose only
/// variable is `name` set to `value`.
fn marked_by(name: &'static str, value: &Result<&'static str, OsString>) -> Mark {
    marked(ToolName::new("shelf"), |read| {
        if read != name {
            return Err(VarError::NotPresent);
        }
        value
            .clone()
            .map(str::to_owned)
            .map_err(VarError::NotUnicode)
    })
}

#[test]
fn a_program_name_in_the_tool_s_program_variable_marks_the_run_with_that_program() {
    assert_eq!(
        marked_by("SHELF_PROGRAM", &Ok("flightdeck")),
        Mark::Program(Program::new("flightdeck").unwrap())
    );
}

#[test]
fn an_upper_case_over_long_or_non_utf_8_program_variable_is_refused() {
    let long: &'static str = "a".repeat(33).leak();

    assert_eq!(marked_by("SHELF_PROGRAM", &Ok("FLIGHTDECK")), Mark::Refused);
    assert_eq!(marked_by("SHELF_PROGRAM", &Ok(long)), Mark::Refused);
    assert_eq!(
        marked_by("SHELF_PROGRAM", &Err(OsString::from("flightdeck"))),
        Mark::Refused
    );
}

#[test]
fn an_unset_or_empty_program_variable_or_another_tool_s_leaves_the_run_unmarked() {
    assert_eq!(marked_by("UNSET", &Ok("flightdeck")), Mark::Unmarked);
    assert_eq!(marked_by("SHELF_PROGRAM", &Ok("")), Mark::Unmarked);
    assert_eq!(
        marked_by("READING_PROGRAM", &Ok("flightdeck")),
        Mark::Unmarked
    );
}

#[test]
fn only_a_refused_mark_has_a_warning_and_it_names_the_variable_and_the_rule() {
    let shelf = ToolName::new("shelf");

    assert_eq!(
        Mark::Refused.warning(shelf).as_deref(),
        Some(
            "warning: SHELF_PROGRAM was ignored, because a program name is 1 to 32 lowercase letters, digits, `-` and `_`, starting with a letter, such as flightdeck; the run is recorded without a program\n"
        )
    );
    assert_eq!(Mark::Unmarked.warning(shelf), None);
    assert_eq!(
        Mark::Program(Program::new("flightdeck").unwrap()).warning(shelf),
        None
    );
}

#[test]
fn a_refused_mark_and_an_unmarked_run_have_no_program() {
    assert_eq!(Mark::Refused.program(), None);
    assert_eq!(Mark::Unmarked.program(), None);
    assert_eq!(
        Mark::Program(Program::new("flightdeck").unwrap()).program(),
        Program::new("flightdeck")
    );
}
