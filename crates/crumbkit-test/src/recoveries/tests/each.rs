//! Tests for a step that runs its call once for each value of a list field
//! of its failure, as a batch's over-budget failure offers.

use snapbox::assert::Action;

use super::{NOT_FOUND, OTHERS, Setup, check_with, problems, recoveries};
use crate::recoveries::Failing;

/// Returns the line of a `tool::books::not_found` failure document with
/// `fields`, a JSON object, whose recovery runs `tool books show <ID>` for
/// each value of its field `left_out`.
fn for_each(fields: &str) -> String {
    format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"tool::books::not_found","message":"m","fields":{fields},"recovery":{{"kind":"run","call":"tool books show <ID>","each":"left_out"}}}}"#
    )
}

#[test]
fn a_step_for_each_value_of_a_list_field_runs_with_the_first_value_as_one_word_and_is_no_finding() {
    let show = for_each(r#"{"left_out":["Le Guin","8"],"budget":480}"#);
    let setup = Setup::new(
        "recoveries-each",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show Le Guin --json", 0, "{}\n", ""),
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
        setup
            .report()
            .contains("  run tool books show <ID> as tool books show 'Le Guin'\n    exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_step_for_each_value_of_a_field_that_holds_no_value_fails_the_check() {
    let show = for_each(r#"{"left_out":[],"unlisted":2}"#);
    let setup = Setup::new(
        "recoveries-each-empty",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books show <ID>` runs once for each value of the failure's field left_out, which is not a list that holds a value"
        )
    );
}
