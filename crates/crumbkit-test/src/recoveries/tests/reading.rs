//! Tests for the recovery check on a tool that prints its failures without
//! a crumbkit document, as a `code:` line.

use crumbkit::help::Guide;
use snapbox::Assert;
use snapbox::assert::Action;

use super::{Answer, CODES, OTHERS, SHOW_7, Setup, tool};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Recoveries, check};
use crate::testing::panic_message;

/// Runs the check of the tool that gives `answers`, with the opt-outs
/// `allow`, and returns the panic message, or `None` when it passes, and
/// the report's directory.
fn checked(answers: &[Answer<'_>], allow: &[Allow]) -> (Option<String>, Setup) {
    let setup = Setup::new("recoveries-reading", answers);
    let examples = Examples::new(&setup.script, "").allow(allow);
    let recoveries = Recoveries::new(examples, CODES)
        .starts(SHOW_7)
        .unprovoked(OTHERS);
    let message = panic_message(|| {
        check(
            &tool(),
            &Guide::new(),
            &recoveries,
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    });
    (message, setup)
}

#[test]
fn the_recovery_check_reads_the_code_line_of_a_failed_run_that_prints_no_document() {
    let answers: &[Answer<'_>] = &[(
        "books show 7 --json",
        1,
        "",
        "error: no book 7\n  code:  tool::books::not_found\n",
    )];

    // A `code:` line gives a failure without steps, which
    // `recoveries/next-step` reports unless an opt-out covers it.
    let allow = [Allow::new(
        Rule::NextStep,
        "books show",
        "The tool prints its recoveries only as text.",
    )];

    let (message, setup) = checked(answers, &allow);

    assert_eq!(message, None);
    assert!(
        setup
            .report()
            .starts_with("tool books show 7\n  fails with tool::books::not_found\n\n"),
        "{}",
        setup.report()
    );
}
