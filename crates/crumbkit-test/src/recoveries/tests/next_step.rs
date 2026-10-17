//! Tests for `recoveries/next-step`, which finds a failure with one of the
//! tool's declared codes that offers an agent no next step.

use crumbkit::Spelling;

use super::{CODES, OTHERS, SHOW_7, Setup, UNPROVOKED, failure, problems, run};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Recoveries, Unprovoked};

/// Returns the panic message of the check of `setup`'s tool, which starts
/// with `SHOW_7`, with the opt-outs `allow` and the listed codes
/// `unprovoked`, or `None` when it passes.
fn checked(setup: &Setup, allow: &[Allow], unprovoked: &[Unprovoked<'_>]) -> Option<String> {
    problems(
        setup,
        &Recoveries::new(
            Examples::new(&setup.script, "").allow(allow),
            Spelling::COLONS,
            CODES,
        )
        .starts(SHOW_7)
        .unprovoked(unprovoked),
    )
}

/// Returns the tool whose `books show 7` fails with `tool::books::not_found`
/// and offers `tool books list`, and whose `books list` fails with `code`,
/// whose failure offers `recovery`, and writes `stderr`.
fn tool(label: &str, code: &str, recovery: &str, stderr: &'static str) -> Setup {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let list = failure(code, recovery);
    Setup::new(
        label,
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, list.as_str(), stderr),
        ],
    )
}

#[test]
fn a_declared_code_whose_failure_offers_no_run_retry_or_hint_is_a_dead_end_unless_an_opt_out_names_the_failing_command()
 {
    let setup = tool(
        "recoveries-dead-end",
        "tool::books::unreadable",
        r#"{"kind":"none","reason":"The disk is full."}"#,
        "",
    );
    let allow = [Allow::new(
        Rule::NextStep,
        "books list",
        "A person must free disk space first.",
    )];

    assert_eq!(
        checked(&setup, &[], UNPROVOKED).as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books list` fails with tool::books::unreadable, \
             whose failure offers no run, retry or hint step, so an agent has no next step. \
             The failure can offer a recovery, or an opt-out from `recoveries/next-step` names \
             `tool books list` with a reason"
        )
    );
    assert_eq!(checked(&setup, &allow, UNPROVOKED), None);
}

#[test]
fn a_crumbkit_code_without_a_next_step_is_no_dead_end_so_an_opt_out_for_it_is_stale() {
    let setup = tool(
        "recoveries-crumbkit-none",
        "tool::confirmation::declined",
        r#"{"kind":"none","reason":"The person said no."}"#,
        "",
    );
    let allow = [Allow::new(
        Rule::NextStep,
        "books list",
        "The person said no.",
    )];

    assert_eq!(
        checked(&setup, &allow, OTHERS).as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             the opt-out from `recoveries/next-step` for `tool books list` excuses no problem, so it is stale"
        )
    );
}

#[test]
fn a_failure_whose_run_warns_that_a_recovery_did_not_render_is_reported_only_by_the_warning() {
    let setup = tool(
        "recoveries-unrendered",
        "tool::books::unreadable",
        r#"{"kind":"none","reason":"The call names no command."}"#,
        "warning: tool left out a recovery\n",
    );

    assert_eq!(
        checked(&setup, &[], UNPROVOKED).as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books list` writes a warning to standard error: \
             warning: tool left out a recovery"
        )
    );
}
