//! Tests for the sources of a placeholder's value: the values of one start,
//! the fields of the failure that offers the step, and the values that
//! every chain shares.

use crumbkit::Spelling;
use snapbox::assert::Action;

use super::{CODES, NOT_FOUND, OTHERS, Setup, check_with, problems, recoveries, run};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries};

/// The opt-out that lets a recovery of `tool books show` fill a placeholder
/// from a field of its failure.
const FIELD_VALUE: &[Allow] = &[Allow::new(
    Rule::FieldValue,
    "books show",
    "The failure's path changes on each run.",
)];

/// Returns the settings that run `setup`'s script with the opt-outs
/// `allow` and check the tool's codes with `starts`.
fn allowing<'a>(setup: &'a Setup, starts: &'a [Failing<'a>], allow: &'a [Allow]) -> Recoveries<'a> {
    Recoveries::new(
        Examples::new(&setup.script, "").allow(allow),
        Spelling::COLONS,
        CODES,
    )
    .starts(starts)
    .unprovoked(OTHERS)
}

/// Returns the line of a `tool::books::not_found` failure document with
/// `fields`, a JSON object, whose recovery runs `call`.
fn with_fields(fields: &str, call: &str) -> String {
    format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"tool::books::not_found","message":"m","fields":{fields},"recovery":{}}}"#,
        run(call)
    )
}

#[test]
fn a_start_s_own_values_fill_the_placeholders_of_its_chain_and_no_other_chain() {
    let show = with_fields("{}", "tool books list <path>");
    let setup = Setup::new(
        "recoveries-start-values",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 8 --json", 1, show.as_str(), ""),
            ("books list own --json", 0, "{}\n", ""),
            ("books list shared --json", 0, "{}\n", ""),
        ],
    );
    let starts = [
        Failing::new(&NOT_FOUND, &["books", "show", "7"]).values(&[("path", "own")]),
        Failing::new(&NOT_FOUND, &["books", "show", "8"]),
    ];

    check_with(
        &setup,
        &recoveries(&setup, &starts, OTHERS).values(&[("path", "shared")]),
        Action::Overwrite,
    );

    assert!(
        setup.report().starts_with(
            "tool books show 7\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list <path> as tool books list own\n\
             \x20   exits 0\n\n\
             tool books show 8\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list <path> as tool books list shared\n\
             \x20   exits 0\n\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_field_of_the_failure_that_offers_a_step_fills_the_placeholder_with_the_same_name() {
    let show = with_fields(r#"{"path":"/tasks/a"}"#, "tool books list <path>");
    let setup = Setup::new(
        "recoveries-field",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list /tasks/a --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    check_with(
        &setup,
        &allowing(&setup, &starts, FIELD_VALUE),
        Action::Overwrite,
    );

    assert!(
        setup
            .report()
            .contains("  run tool books list <path> as tool books list /tasks/a\n    <path> from the failure's field path\n    exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_field_fills_a_placeholder_whose_name_differs_only_in_capitals_or_a_dash_for_an_underscore() {
    let show = with_fields(
        r#"{"book_id":7,"kept":true}"#,
        "tool books list <BOOK-ID> <kept>",
    );
    let setup = Setup::new(
        "recoveries-field-case",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list 7 true --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    check_with(
        &setup,
        &allowing(&setup, &starts, FIELD_VALUE),
        Action::Overwrite,
    );

    assert!(
        setup.report().contains(
            "  run tool books list <BOOK-ID> <kept> as tool books list 7 true\n    \
             <BOOK-ID> from the failure's field book_id\n    \
             <kept> from the failure's field kept\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_start_s_own_value_wins_over_a_field_and_a_field_wins_over_a_shared_value() {
    let show = with_fields(
        r#"{"path":"field","id":"field"}"#,
        "tool books list <path> <id> <other>",
    );
    let setup = Setup::new(
        "recoveries-value-order",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list own field shared --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"]).values(&[("path", "own")])];
    let shared = [("path", "shared"), ("id", "shared"), ("other", "shared")];

    check_with(
        &setup,
        &allowing(&setup, &starts, FIELD_VALUE).values(&shared),
        Action::Overwrite,
    );

    assert!(
        setup.report().contains(
            "  run tool books list <path> <id> <other> as tool books list own field shared\n    \
             <id> from the failure's field id\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_placeholder_that_a_field_of_the_failure_fills_fails_the_check_naming_the_field_and_call_value()
{
    let show = with_fields(r#"{"path":"/tasks/a"}"#, "tool books list <path>");
    let setup = Setup::new(
        "recoveries-field-finding",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list /tasks/a --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 7: the step `tool books list <path>` takes `<path>` from the failure's field path, so an agent must substitute the value itself. The failure's recovery can carry the value with Call::value, or an opt-out from `recoveries/field-value` names `tool books show` with a reason"
        )
    );
}

#[test]
fn an_opt_out_from_field_value_for_the_failing_command_excuses_the_fill_and_the_report_still_shows_it()
 {
    let show = with_fields(r#"{"path":"/tasks/a"}"#, "tool books list <path>");
    let setup = Setup::new(
        "recoveries-field-allowed",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list /tasks/a --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &allowing(&setup, &starts, FIELD_VALUE));

    assert_eq!(message, None);
    assert!(
        setup.report().starts_with(
            "tool books show 7\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list <path> as tool books list /tasks/a\n\
             \x20   <path> from the failure's field path\n\
             \x20   exits 0\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn an_opt_out_from_field_value_that_excuses_no_fill_is_stale() {
    let show = with_fields("{}", "tool books list");
    let setup = Setup::new(
        "recoveries-field-stale",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];

    let message = problems(&setup, &allowing(&setup, &starts, FIELD_VALUE));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             the opt-out from `recoveries/field-value` for `tool books show` excuses no problem, so it is stale"
        )
    );
}

#[test]
fn a_start_s_own_value_that_wins_over_a_field_is_no_finding() {
    let show = with_fields(r#"{"path":"field"}"#, "tool books list <path>");
    let setup = Setup::new(
        "recoveries-field-own",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list own --json", 0, "{}\n", ""),
        ],
    );
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"]).values(&[("path", "own")])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(message, None);
    assert!(
        !setup.report().contains("from the failure's field"),
        "{}",
        setup.report()
    );
}
