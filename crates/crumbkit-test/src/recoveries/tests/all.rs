//! Tests for a step that runs its call once with every value of a list
//! field of its failure, as a batch's over-budget failure offers.

use snapbox::assert::Action;

use super::{NOT_FOUND, OTHERS, Setup, check_with, problems, recoveries};
use crate::recoveries::Failing;

/// Returns the line of a `tool::books::not_found` failure document with
/// `fields`, a JSON object, whose recovery runs `tool books show <ID>...`
/// with every value of its field `left_out`.
fn with_all(fields: &str) -> String {
    format!(
        r#"{{"schema":"tool/failure/v1","code":"tool::books::not_found","message":"m","fields":{fields},"recovery":{{"kind":"run","call":"tool books show <ID>...","all":"left_out"}}}}"#
    )
}

#[test]
fn a_step_with_every_value_of_a_list_field_runs_once_with_each_value_as_its_own_word_and_is_no_finding()
 {
    let show = with_all(r#"{"left_out":["Le Guin","8"],"budget":480}"#);
    let setup = Setup::new(
        "recoveries-all",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show Le Guin 8 --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];
    let shared = [("ID", "shared")];

    check_with(
        &setup,
        &recoveries(&setup, &starts, OTHERS).values(&shared),
        Action::Overwrite,
    );

    assert!(
        setup.report().contains(
            "  run tool books show <ID>... as tool books show 'Le Guin' 8\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_step_with_every_value_of_a_field_that_holds_no_value_fails_the_check() {
    let show = with_all(r#"{"left_out":[],"unlisted":2}"#);
    let setup = Setup::new(
        "recoveries-all-empty",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books show <ID>...` runs with every value of the failure's field left_out, which is not a list that holds a value"
        )
    );
}

#[test]
fn a_step_with_every_value_that_fails_again_with_the_same_code_and_fewer_values_left_makes_progress()
 {
    let first = with_all(r#"{"left_out":["8","9"]}"#);
    let second = with_all(r#"{"left_out":["9"]}"#);
    let setup = Setup::new(
        "recoveries-all-progress",
        &[
            ("books show 7 --json", 1, first.as_str(), ""),
            ("books show 8 9 --json", 1, second.as_str(), ""),
            ("books show 9 --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(message, None);
    assert!(
        setup
            .report()
            .contains("    run tool books show <ID>... as tool books show 9\n      exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_step_with_every_value_that_fails_again_with_as_many_values_left_learns_nothing_new() {
    let show = with_all(r#"{"left_out":["8","9"]}"#);
    let setup = Setup::new(
        "recoveries-all-stuck",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 8 9 --json", 1, show.as_str(), ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books show <ID>...` fails again with tool::books::not_found, so an agent that follows the recovery learns nothing new"
        )
    );
}
