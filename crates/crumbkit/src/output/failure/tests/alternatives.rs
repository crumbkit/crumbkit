//! Tests for how `finish_to` prints a failure's alternative repairs.

use super::{Missing, fail_with, missing, run_list};
use crate::{Call, Recovery};

/// Returns a failure whose recovery is `recovery` and whose alternatives are
/// `alternatives`.
fn with_alternatives(recovery: Recovery, alternatives: Vec<Recovery>) -> Missing {
    Missing {
        alternatives,
        ..missing(recovery)
    }
}

/// Returns a recovery that names a command the tool does not have.
fn run_remove() -> Recovery {
    Recovery::Run(Call::new(&["remove"]))
}

#[test]
fn each_alternative_prints_as_one_help_line_after_the_preferred_recovery() {
    let failure = with_alternatives(
        run_list(),
        vec![
            Recovery::Hint("set SOLAR_TOKEN".to_owned()),
            Recovery::Retry,
        ],
    );

    let finished = fail_with(&[], failure);

    assert_eq!(
        finished.err,
        concat!(
            "error: the solar system has no planet Vulcan\n",
            "  help:  run `tool list`\n",
            "  help:  set SOLAR_TOKEN\n",
            "  help:  run the same command again later\n",
            "  code:  solar::planets::not_found\n",
        )
    );
}

#[test]
fn json_holds_the_alternatives_after_the_recovery_tagged_by_kind() {
    let failure = with_alternatives(
        run_list(),
        vec![
            Recovery::Hint("set SOLAR_TOKEN".to_owned()),
            Recovery::Retry,
        ],
    );

    let finished = fail_with(&["--json"], failure);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"},"alternatives":[{"kind":"hint","text":"set SOLAR_TOKEN"},{"kind":"retry"}]}"#,
            "\n"
        )
    );
}

#[test]
fn an_alternative_that_offers_no_step_is_left_out() {
    let failure = || {
        with_alternatives(
            run_list(),
            vec![
                run_remove(),
                Recovery::None {
                    reason: "nothing else works".to_owned(),
                },
                Recovery::Retry,
            ],
        )
    };

    let text = fail_with(&[], failure());
    let json = fail_with(&["--json"], failure());

    assert!(
        text.err.contains(
            "  help:  run `tool list`\n  help:  run the same command again later\n  code:"
        ),
        "{}",
        text.err
    );
    assert!(
        json.out.contains(
            r#""recovery":{"kind":"run","call":"tool list"},"alternatives":[{"kind":"retry"}]}"#
        ),
        "{}",
        json.out
    );
}

#[test]
fn the_first_alternative_that_offers_a_step_replaces_a_preferred_recovery_that_offers_none() {
    let failure = || {
        with_alternatives(
            run_remove(),
            vec![
                run_remove(),
                Recovery::Hint("check the spelling".to_owned()),
                Recovery::Retry,
            ],
        )
    };

    let text = fail_with(&[], failure());
    let json = fail_with(&["--json"], failure());

    assert!(
        text.err.contains(
            "  help:  check the spelling\n  help:  run the same command again later\n  code:"
        ),
        "{}",
        text.err
    );
    assert!(
        json.out.contains(
            r#""recovery":{"kind":"hint","text":"check the spelling"},"alternatives":[{"kind":"retry"}]}"#
        ),
        "{}",
        json.out
    );
}

#[test]
fn a_preferred_recovery_that_offers_no_step_stays_when_no_alternative_offers_one() {
    let failure = with_alternatives(run_remove(), vec![run_remove()]);

    let finished = fail_with(&["--json"], failure);

    assert!(
        finished.out.contains(
            r#""recovery":{"kind":"none","reason":"the suggested command is not available"}}"#
        ),
        "{}",
        finished.out
    );
}
