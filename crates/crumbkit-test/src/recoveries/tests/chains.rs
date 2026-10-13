//! Tests for how a chain follows the failures that one run prints and
//! judges a code that an earlier failure of the chain had already.

use crumbkit::Spelling;

use super::{CODES, SHOW_7, Setup, UNPROVOKED, failure, problems, recoveries, run};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries};

/// The start that fails on books 7 and 8, which a batch reports one
/// failure each.
const SHOW_7_8: &[Failing<'_>] = &[Failing::new(
    &super::NOT_FOUND,
    &["books", "show", "7", "8"],
)];

/// Returns a batch document whose `failures` hold one entry for each code
/// in `codes`, each with the recovery that runs `tool books list`.
fn batch(codes: &[&str]) -> String {
    let entries: Vec<String> = codes
        .iter()
        .map(|code| {
            format!(
                r#"{{"code":"{code}","message":"m","recovery":{}}}"#,
                run("tool books list")
            )
        })
        .collect();
    format!(
        r#"{{"schema":"tool/books-show/v1","data":[],"failures":[{}]}}"#,
        entries.join(",")
    )
}

#[test]
fn every_failure_of_a_batch_starts_its_own_branch() {
    let batch = batch(&["tool::books::not_found", "tool::books::unreadable"]);
    let setup = Setup::new(
        "recoveries-branches",
        &[
            ("books show 7 8 --json", 1, batch.as_str(), ""),
            ("books list --json", 0, "{}\n", ""),
        ],
    );

    let message = problems(&setup, &recoveries(&setup, SHOW_7_8, UNPROVOKED));

    assert_eq!(message, None);
    assert!(
        setup.report().starts_with(
            "tool books show 7 8\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list\n\
             \x20   exits 0\n\
             \x20 fails with tool::books::unreadable\n\
             \x20 run tool books list\n\
             \x20   exits 0\n\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_start_passes_when_any_of_its_failures_carries_its_code() {
    let found = batch(&["tool::books::unreadable", "tool::books::not_found"]);
    let missing = batch(&["tool::books::unreadable", "tool::books::removed_file"]);
    let setup = Setup::new(
        "recoveries-any",
        &[
            ("books show 7 8 --json", 1, found.as_str(), ""),
            ("books show 9 --json", 1, missing.as_str(), ""),
            ("books list --json", 0, "{}\n", ""),
        ],
    );
    let starts = [
        SHOW_7_8[0],
        Failing::new(&super::NOT_FOUND, &["books", "show", "9"]),
    ];

    let message = problems(&setup, &recoveries(&setup, &starts, &[]));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 9: the start must fail with tool::books::not_found, and it fails with tool::books::unreadable, tool::books::removed_file"
        )
    );
}

#[test]
fn a_step_that_fails_with_the_code_of_any_earlier_failure_in_its_chain_fails_the_check_unless_an_opt_out_covers_it()
 {
    let show = format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"tool::books::not_found","message":"m","recovery":{},"alternatives":[{}]}}"#,
        run("tool books show 9"),
        run("tool books list")
    );
    let list = failure("tool::books::unreadable", &run("tool books show 8"));
    let again = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-ancestor",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 9 --json", 1, again.as_str(), ""),
            ("books list --json", 1, list.as_str(), ""),
            ("books show 8 --json", 1, again.as_str(), ""),
        ],
    );
    let allow = [Allow::new(
        Rule::SameCode,
        "books show",
        "An agent fixes the ids file by hand before it shows the book.",
    )];

    let message = problems(&setup, &recoveries(&setup, SHOW_7, UNPROVOKED));
    let allowed = problems(
        &setup,
        &Recoveries::new(
            Examples::new(&setup.script, "").allow(&allow),
            Spelling::COLONS,
            CODES,
        )
        .starts(SHOW_7)
        .unprovoked(UNPROVOKED),
    );

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books show 9` fails again with tool::books::not_found, so an agent that follows the recovery learns nothing new\n\
             tool books show 7: the step `tool books show 8` fails again with tool::books::not_found, so an agent that follows the recovery learns nothing new"
        )
    );
    assert_eq!(allowed, None);
}
