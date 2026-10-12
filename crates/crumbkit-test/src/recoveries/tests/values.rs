//! Tests for the sources of a placeholder's value: the values of one start,
//! the fields of the failure that offers the step, and the values that
//! every chain shares.

use snapbox::assert::Action;

use super::{NOT_FOUND, OTHERS, Setup, check_with, recoveries, run};
use crate::recoveries::Failing;

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
        &recoveries(&setup, &starts, OTHERS),
        Action::Overwrite,
    );

    assert!(
        setup
            .report()
            .contains("  run tool books list <path> as tool books list /tasks/a\n    exits 0\n"),
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
        &recoveries(&setup, &starts, OTHERS),
        Action::Overwrite,
    );

    assert!(
        setup.report().contains(
            "  run tool books list <BOOK-ID> <kept> as tool books list 7 true\n    exits 0\n"
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
        &recoveries(&setup, &starts, OTHERS).values(&shared),
        Action::Overwrite,
    );

    assert!(
        setup.report().contains(
            "  run tool books list <path> <id> <other> as tool books list own field shared\n    exits 0\n"
        ),
        "{}",
        setup.report()
    );
}
