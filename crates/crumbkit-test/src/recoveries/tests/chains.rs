//! Tests for how a chain judges the failures of its steps: a code that an
//! earlier failure of the chain had already.

use crumbkit::Spelling;

use super::{CODES, SHOW_7, Setup, UNPROVOKED, failure, problems, recoveries, run};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::Recoveries;

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
