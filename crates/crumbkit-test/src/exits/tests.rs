//! Tests for the exit code check.

use clap::Command;
use crumbkit::Code;
use crumbkit::help::{Exit, Guide, Rename};

use super::check_exit_codes;
use crate::testing::panic_message;
use crate::{Allow, Rule};

/// A tool with one resource that has two verbs.
fn shelf() -> Command {
    Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("import")),
    )
}

/// Returns the message of the check of `guide` with `allow`, or `None`
/// when the check passes.
fn checked(guide: &Guide, allow: &[Allow]) -> Option<String> {
    panic_message(|| check_exit_codes(&shelf(), guide, allow))
}

#[test]
fn codes_from_3_to_125_with_one_line_meanings_and_real_commands_pass() {
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(3, "The catalogue was locked by another run."),
        Exit::new(125, "The download ran out of time.").commands(&[&["books", "import"]]),
    ]);

    check_exit_codes(&shelf(), &GUIDE, &[]);
}

#[test]
fn a_tool_without_exit_codes_passes() {
    check_exit_codes(&shelf(), &Guide::new(), &[]);
}

#[test]
fn the_exit_code_0_fails_the_check_and_an_opt_out_does_not_excuse_it() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(0, "Done.")]);
    const ALLOW: &[Allow] = &[Allow::new(Rule::SharedExit, "", "Old scripts.")];

    assert_eq!(
        checked(&GUIDE, ALLOW).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 0 means that the run succeeded, so a caller cannot tell a failure from a success\n\
         the opt-out from `exit-codes/shared-exit` for `shelf` excuses no problem, so it is stale"
    );
}

#[test]
fn the_exit_codes_1_and_2_fail_the_check_because_crumbkit_gives_them_a_meaning() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(1, "Failed."), Exit::new(2, "Refused.")]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 1 already means that crumbkit's default failure ended the run, so it has two meanings\n\
         the exit code 2 already means that crumbkit refused the command line, so it has two meanings"
    );
}

#[test]
fn an_opt_out_from_the_shared_exit_rule_excuses_the_exit_code_1_or_2() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const GUIDE: Guide =
        Guide::new().exits(&[Exit::new(1, "The catalogue was locked.").failures(&[&LOCKED])]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::SharedExit,
        "",
        "Scripts read 1 as locked.",
    )];

    assert_eq!(checked(&GUIDE, ALLOW), None);
}

#[test]
fn a_code_from_126_fails_the_check() {
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(126, "The cache is not writable."),
        Exit::new(255, "Gave up."),
    ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 126 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 126 from the shell's\n\
         the exit code 255 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 255 from the shell's"
    );
}

#[test]
fn an_opt_out_excuses_a_code_from_126_for_every_command_that_returns_it() {
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(130, "Interrupted.").commands(&[&["books", "import"]]),
        Exit::new(137, "Killed."),
    ]);
    const ALLOW: &[Allow] = &[
        Allow::new(Rule::ShellExitCode, "books", "Import mirrors the shell."),
        Allow::new(Rule::ShellExitCode, "", "Scripts read 137."),
    ];

    assert_eq!(checked(&GUIDE, ALLOW), None);
}

#[test]
fn an_opt_out_for_one_command_does_not_excuse_a_code_that_every_command_returns() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(130, "Interrupted.")]);
    const ALLOW: &[Allow] = &[Allow::new(Rule::ShellExitCode, "books", "Import only.")];

    assert_eq!(
        checked(&GUIDE, ALLOW).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 130 is one that a shell reports for its own reasons, so a caller cannot tell the tool's 130 from the shell's\n\
         the opt-out from `exit-codes/shell-code` for `shelf books` excuses no problem, so it is stale"
    );
}

#[test]
fn two_declarations_of_one_code_fail_the_check() {
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(3, "Locked."),
        Exit::new(3, "Busy."),
        Exit::new(3, "Taken."),
    ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 is declared more than once, so a caller cannot tell which meaning applies"
    );
}

#[test]
fn a_meaning_that_is_empty_or_longer_than_one_line_fails_the_check() {
    const GUIDE: Guide =
        Guide::new().exits(&[Exit::new(3, " "), Exit::new(4, "Locked.\nTry again.")]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 has no meaning\n\
         the meaning of the exit code 4 is more than one line, and help and `commands list` print it on one"
    );
}

#[test]
fn a_command_path_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(3, "Locked.").commands(&[&["books", "export"], &["books"]])]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 3 names `shelf books export`, which is no command"
    );
}

#[test]
fn a_failure_code_that_two_exits_list_fails_the_check() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(3, "Locked.").failures(&[&LOCKED]),
        Exit::new(4, "Busy.").failures(&[&LOCKED]),
        Exit::new(5, "Taken.").failures(&[&LOCKED]),
    ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 4 lists the failure code shelf::books::locked, which the exit code 3 lists already, so a run that fails with it cannot exit with both"
    );
}

#[test]
fn an_exit_that_lists_one_of_crumbkit_s_failure_codes_passes() {
    const GUIDE: Guide =
        Guide::new().exits(&[Exit::new(64, "Refused.").failures(&[&crumbkit::codes::CLI_USAGE])]);

    assert_eq!(checked(&GUIDE, &[]), None);
}

#[test]
fn an_exit_that_lists_a_crumbkit_code_and_another_that_lists_its_rename_fail_the_check() {
    const USAGE: Code = Code::new("shelf", "cli", "usage");
    const GUIDE: Guide = Guide::new()
        .codes(&[Rename::new(&crumbkit::codes::CLI_USAGE, &USAGE)])
        .exits(&[
            Exit::new(64, "Refused.").failures(&[&crumbkit::codes::CLI_USAGE]),
            Exit::new(65, "Usage.").failures(&[&USAGE]),
        ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 65 lists the failure code shelf::cli::usage, which the exit code 64 lists already, so a run that fails with it cannot exit with both"
    );
}

#[test]
fn a_crumbkit_code_that_two_renames_name_fails_the_check() {
    const USAGE: Code = Code::new("shelf", "cli", "usage");
    const BAD: Code = Code::new("shelf", "cli", "bad");
    const GUIDE: Guide = Guide::new().codes(&[
        Rename::new(&crumbkit::codes::CLI_USAGE, &USAGE),
        Rename::new(&crumbkit::codes::CLI_USAGE, &BAD),
    ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the guide renames crumbkit::cli::usage twice, and a run prints only the first rename, shelf::cli::usage"
    );
}

#[test]
fn two_renames_to_one_code_of_the_tool_fail_the_check() {
    const USAGE: Code = Code::new("shelf", "cli", "usage");
    const GUIDE: Guide = Guide::new().codes(&[
        Rename::new(&crumbkit::codes::CLI_USAGE, &USAGE),
        Rename::new(&crumbkit::codes::COMMANDS_NOT_FOUND, &USAGE),
    ]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the guide renames crumbkit::commands::not_found to shelf::cli::usage, which it gives crumbkit::cli::usage already, so a caller cannot tell the two failures apart"
    );
}

#[test]
fn exits_that_list_different_failure_codes_pass() {
    const LOCKED: Code = Code::new("shelf", "books", "locked");
    const SLOW: Code = Code::new("shelf", "books", "timed_out");
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(3, "Locked.").failures(&[&LOCKED]),
        Exit::new(124, "Slow.").failures(&[&SLOW]),
    ]);

    assert_eq!(checked(&GUIDE, &[]), None);
}
