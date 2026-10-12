//! Tests for the exit code check.

use clap::Command;
use crumbkit::help::{Exit, Guide};

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
fn a_code_that_crumbkit_owns_fails_the_check() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(0, "Done."), Exit::new(2, "Refused.")]);

    assert_eq!(
        checked(&GUIDE, &[]).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 0 is one of crumbkit's codes 0, 1 and 2, which `shelf help exit-codes` already describes\n\
         the exit code 2 is one of crumbkit's codes 0, 1 and 2, which `shelf help exit-codes` already describes"
    );
}

#[test]
fn an_opt_out_does_not_excuse_a_code_that_crumbkit_owns() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(1, "Failed.")]);
    const ALLOW: &[Allow] = &[Allow::new(Rule::ShellExitCode, "", "Old scripts.")];

    assert_eq!(
        checked(&GUIDE, ALLOW).unwrap(),
        "the command tree breaks crumbkit's exit code rules:\n\
         the exit code 1 is one of crumbkit's codes 0, 1 and 2, which `shelf help exit-codes` already describes\n\
         the opt-out from `exit-codes/shell-code` for `shelf` excuses no problem, so it is stale"
    );
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
