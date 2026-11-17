//! Tests for how `finish_to` prints a recovery that names another program's
//! command.

use super::{Missing, fail_with, missing, run_list};
use crate::{Program, Recovery};

const GIT: Program = Program::new("git");

#[test]
fn a_program_recovery_prints_its_command_line_on_the_help_line() {
    let finished = fail_with(&[], missing(Recovery::program(GIT, ["status"])));

    assert_eq!(
        finished.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `git status`\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

#[test]
fn a_program_recovery_prints_as_a_program_step_in_json() {
    let finished = fail_with(
        &["--json"],
        missing(Recovery::program(GIT, ["commit", "-m", "Add books"])),
    );

    assert!(
        finished.out.contains(
            r#""recovery":{"kind":"program","program":"git","args":["commit","-m","Add books"],"run":"git commit -m 'Add books'"}"#
        ),
        "{}",
        finished.out
    );
}

#[test]
fn a_program_argument_that_a_shell_would_split_prints_between_single_quotes() {
    let finished = fail_with(
        &[],
        missing(Recovery::program(GIT, ["log", "--grep=two words"])),
    );

    assert!(
        finished
            .err
            .contains("\n  help:  run `git log '--grep=two words'`\n"),
        "{}",
        finished.err
    );
}

// A release build prints the same failure without the warning.
#[cfg(debug_assertions)]
#[test]
fn a_program_argument_with_a_control_character_prints_no_next_step_and_warns_in_a_debug_build() {
    let finished = fail_with(&[], missing(Recovery::program(GIT, ["a\nb"])));

    assert!(
        finished
            .err
            .contains("\n  help:  no next step: the suggested command is not available\n"),
        "{}",
        finished.err
    );
    assert!(
        finished.err.starts_with(
            "warning: tool left out a recovery of the failure solar::planets::not_found, because "
        ),
        "{}",
        finished.err
    );
}

#[test]
fn a_program_alternative_prints_after_the_recovery() {
    let failure = Missing {
        alternatives: vec![Recovery::program(GIT, ["status"])],
        ..missing(run_list())
    };

    let text = fail_with(&[], failure);

    assert_eq!(
        text.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list`\n",
            "  help:  run `git status`\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}
