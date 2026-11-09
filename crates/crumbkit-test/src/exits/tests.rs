//! Tests for the exit code check.

use clap::Command;
use crumbkit::tool::Exit;
use crumbkit::{Code, Tool};

use super::check_exit_codes;
use crate::testing::panic_message;
use crate::{Allow, Rule};
use crumbkit::tool::Failures;

/// A tool with one resource that has two verbs.
fn shelf() -> Command {
    Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("import")),
    )
}

/// Returns the message of the check of `tool` with `allow`, or `None`
/// when the check passes.
fn checked(tool: &Tool, allow: &[Allow]) -> Option<String> {
    panic_message(|| check_exit_codes(&shelf(), tool, allow))
}

#[test]
fn codes_from_3_to_125_with_one_line_meanings_and_real_commands_pass() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(3, "The catalogue was locked by another run."),
        Exit::new(125, "The download ran out of time.").commands(&[&["books", "import"]]),
    ]));

    check_exit_codes(&shelf(), &TOOL, &[]);
}

#[test]
fn a_tool_without_exit_codes_passes() {
    check_exit_codes(&shelf(), &Tool::new(), &[]);
}

#[test]
fn the_exit_code_0_fails_the_check_and_an_opt_out_does_not_excuse_it() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[Exit::new(0, "Done.")]));
    const ALLOW: &[Allow] = &[Allow::new(Rule::SharedExit, "", "Old scripts.")];

    assert_eq!(
        checked(&TOOL, ALLOW).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 0 means that the run succeeded, so a caller cannot tell a failure from a success\n\
         the opt-out from `exit-codes/shared-exit` for `shelf` excuses no problem, so it is stale"
    );
}

#[test]
fn the_exit_codes_1_and_2_fail_the_check_because_crumbkit_gives_them_a_meaning() {
    const TOOL: Tool = Tool::new()
        .failures(Failures::new().exits(&[Exit::new(1, "Failed."), Exit::new(2, "Refused.")]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 1 already means that crumbkit's default failure ended the run, so it has two meanings\n\
         the exit code 2 already means that crumbkit refused the command line, so it has two meanings"
    );
}

#[test]
fn an_opt_out_from_the_shared_exit_rule_excuses_the_exit_code_1_or_2() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const TOOL: Tool = Tool::new().failures(
        Failures::new().exits(&[Exit::new(1, "The catalogue was locked.").failures(&[&LOCKED])]),
    );
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::SharedExit,
        "",
        "Scripts read 1 as locked.",
    )];

    assert_eq!(checked(&TOOL, ALLOW), None);
}

#[test]
fn a_single_code_from_126_to_128_or_from_160_fails_the_check() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(126, "The cache is not writable."),
        Exit::new(128, "Stopped."),
        Exit::new(160, "Gave up."),
        Exit::new(255, "Gave up for good."),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 126 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 126 from the shell's\n\
         the exit code 128 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 128 from the shell's\n\
         the exit code 160 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 160 from the shell's\n\
         the exit code 255 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 255 from the shell's"
    );
}

#[test]
fn a_single_code_for_a_run_that_a_signal_ended_passes() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(129, "The terminal closed during the run."),
        Exit::new(130, "Ctrl-C ended the run.").commands(&[&["books", "import"]]),
        Exit::new(159, "A signal 31 ended the run."),
    ]));

    assert_eq!(checked(&TOOL, &[]), None);
}

#[test]
fn an_opt_out_excuses_a_shell_code_for_every_command_that_returns_it() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(127, "The importer was not found.").commands(&[&["books", "import"]]),
        Exit::new(200, "Gave up."),
    ]));
    const ALLOW: &[Allow] = &[
        Allow::new(Rule::ShellExitCode, "books", "Import mirrors the shell."),
        Allow::new(Rule::ShellExitCode, "", "Scripts read 200."),
    ];

    assert_eq!(checked(&TOOL, ALLOW), None);
}

#[test]
fn an_opt_out_for_one_command_does_not_excuse_a_code_that_every_command_returns() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[Exit::new(127, "Not found.")]));
    const ALLOW: &[Allow] = &[Allow::new(Rule::ShellExitCode, "books", "Import only.")];

    assert_eq!(
        checked(&TOOL, ALLOW).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 127 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 127 from the shell's\n\
         the opt-out from `exit-codes/shell-code` for `shelf books` excuses no problem, so it is stale"
    );
}

#[test]
fn a_range_from_3_up_to_255_passes_without_an_opt_out() {
    const TOOL: Tool = Tool::new().failures(
        Failures::new().exits(
            &[Exit::range(3, 255, "The importer ended with its own code.")
                .commands(&[&["books", "import"]])],
        ),
    );

    assert_eq!(checked(&TOOL, &[]), None);
}

#[test]
fn a_range_that_starts_below_3_or_ends_before_it_starts_fails_the_check() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::range(2, 9, "The importer ended with its own code."),
        Exit::range(30, 20, "The suite ended with its own code."),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit range 2-9 starts below 3, and 0, 1 and 2 already mean success, crumbkit's default failure and a refused command line\n\
         the exit range 30-20 ends before it starts, so it holds no code"
    );
}

#[test]
fn a_range_that_overlaps_a_single_code_or_another_range_fails_the_check() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(124, "The download ran out of time."),
        Exit::range(100, 125, "The importer ended with its own code."),
        Exit::range(3, 100, "The suite ended with its own code."),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit range 100-125 overlaps the exit code 124, so a caller cannot tell which meaning applies\n\
         the exit range 3-100 overlaps the exit range 100-125, so a caller cannot tell which meaning applies"
    );
}

#[test]
fn a_range_that_lists_failure_codes_fails_the_check() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::range(64, 78, "The importer ended with its own code.").failures(&[&LOCKED]),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit range 64-78 lists failure codes, and a run that fails with one exits with 1, because a range names no single code to exit with"
    );
}

#[test]
fn two_declarations_of_one_code_fail_the_check() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(3, "Locked."),
        Exit::new(3, "Busy."),
        Exit::new(3, "Taken."),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 is declared more than once, so a caller cannot tell which meaning applies"
    );
}

#[test]
fn a_meaning_that_is_empty_or_longer_than_one_line_fails_the_check() {
    const TOOL: Tool = Tool::new()
        .failures(Failures::new().exits(&[Exit::new(3, " "), Exit::new(4, "Locked.\nTry again.")]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 has no meaning\n\
         the meaning of the exit code 4 is more than one line, and help and `commands list` print it on one"
    );
}

#[test]
fn a_command_path_that_names_no_command_fails_the_check() {
    const TOOL: Tool = Tool::new().failures(
        Failures::new()
            .exits(&[Exit::new(3, "Locked.").commands(&[&["books", "export"], &["books"]])]),
    );

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 names `shelf books export`, which is no command"
    );
}

#[test]
fn a_failure_code_that_two_exits_list_fails_the_check() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(3, "Locked.").failures(&[&LOCKED]),
        Exit::new(4, "Busy.").failures(&[&LOCKED]),
        Exit::new(5, "Taken.").failures(&[&LOCKED]),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 4 lists the failure code shelf::books::locked, which the exit code 3 lists already, so a run that fails with it cannot exit with both"
    );
}

#[test]
fn an_exit_that_lists_one_of_crumbkit_s_failure_codes_passes() {
    const TOOL: Tool = Tool::new().failures(
        Failures::new()
            .exits(&[Exit::new(64, "Refused.").failures(&[&crumbkit::codes::CLI_USAGE])]),
    );

    assert_eq!(checked(&TOOL, &[]), None);
}

#[test]
fn exits_that_list_different_failure_codes_pass() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const SLOW: Code = Code::new("shelf", "books", "timed_out");
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(3, "Locked.").failures(&[&LOCKED]),
        Exit::new(124, "Slow.").failures(&[&SLOW]),
    ]));

    assert_eq!(checked(&TOOL, &[]), None);
}

#[test]
fn the_problems_spell_crumbkit_s_codes_with_the_tool_s_name() {
    const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(64, "Refused.").failures(&[&crumbkit::codes::CLI_USAGE]),
        Exit::new(65, "Bad.").failures(&[&crumbkit::codes::CLI_USAGE]),
    ]));

    assert_eq!(
        checked(&TOOL, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 65 lists the failure code shelf::cli::usage, which the exit code 64 lists already, so a run that fails with it cannot exit with both"
    );
}
