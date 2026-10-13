//! Tests for the exit codes that the recovery check compares with the
//! exits that the guide declares for failure codes (ADR-0037).

use crumbkit::help::{Exit, Guide, Leaf};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{
    NOT_FOUND, OTHERS, SHOW_7, Setup, UNPROVOKED, Unprovoked, failure, recoveries, run, tool,
};
use crate::recoveries::check;
use crate::testing::panic_message;

/// The exit code 4, which `NOT_FOUND` ends a run with.
const EXITS: &[Exit] = &[Exit::new(4, "No such book.").failures(&[&NOT_FOUND])];

/// A guide that lists `NOT_FOUND` for the exit code 4.
const GUIDE: Guide = Guide::new().exits(EXITS);

/// Returns the panic message of the check of `SHOW_7` with `GUIDE` on a
/// script that answers `books show 7` with `show_exit` and `books list`
/// with `list`, with the listed codes `unprovoked`, or `None` when it
/// passes.
fn checked(
    label: &str,
    show_exit: u8,
    list: (u8, &str),
    unprovoked: &[Unprovoked<'_>],
) -> Option<String> {
    checked_with(&GUIDE, label, show_exit, list, unprovoked)
}

/// Returns the panic message of the check as [`checked`] does, with
/// `guide`.
fn checked_with(
    guide: &Guide,
    label: &str,
    show_exit: u8,
    list: (u8, &str),
    unprovoked: &[Unprovoked<'_>],
) -> Option<String> {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let setup = Setup::new(
        label,
        &[
            ("books show 7 --json", show_exit, show.as_str(), ""),
            ("books list --json", list.0, list.1, ""),
        ],
    );
    panic_message(|| {
        check(
            &tool(),
            guide,
            &recoveries(&setup, SHOW_7, unprovoked),
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    })
}

#[test]
fn a_start_that_exits_with_the_code_that_lists_its_failure_passes() {
    assert_eq!(
        checked("recoveries-exits-pass", 4, (0, "{}\n"), OTHERS),
        None
    );
}

#[test]
fn a_start_whose_failure_an_exit_lists_and_that_exits_with_another_code_fails_the_check() {
    assert_eq!(
        checked("recoveries-exits-start", 1, (0, "{}\n"), OTHERS).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the start fails with tool::books::not_found, which the guide's exit code 4 lists, and it ends with exit status: 1"
    );
}

#[test]
fn a_step_whose_failure_no_exit_lists_fails_the_check_when_it_exits_with_a_listing_code() {
    let unreadable = failure("tool::books::unreadable", r#"{"kind":"retry"}"#);

    assert_eq!(
        checked("recoveries-exits-step", 4, (4, &unreadable), UNPROVOKED).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books list` fails with tool::books::unreadable, which no exit code of the guide lists, and it exits with 4, which the guide declares for the failure codes that it lists"
    );
}

#[test]
fn a_step_whose_failure_no_exit_lists_fails_the_check_when_it_exits_with_another_code_than_1() {
    let unreadable = failure("tool::books::unreadable", r#"{"kind":"retry"}"#);

    assert_eq!(
        checked("recoveries-exits-default", 4, (5, &unreadable), UNPROVOKED).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books list` fails with tool::books::unreadable, which no exit code of the guide lists, and it ends with exit status: 5, where such a failure exits with code 1. A command that passes on the exit code of a program it runs declares it with `Leaf::passes_on_exit`"
    );
}

#[test]
fn a_step_of_a_command_that_passes_on_a_program_s_exit_code_may_end_with_that_code() {
    const PASSING: Guide = Guide::new()
        .exits(EXITS)
        .leaves(&[Leaf::new(&["books", "list"]).passes_on_exit("the catalogue reader")]);
    let unreadable = failure("tool::books::unreadable", r#"{"kind":"retry"}"#);

    assert_eq!(
        checked_with(
            &PASSING,
            "recoveries-exits-passing",
            4,
            (5, &unreadable),
            UNPROVOKED
        ),
        None
    );
}
