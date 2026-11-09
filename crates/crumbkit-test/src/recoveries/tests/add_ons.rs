//! Tests for what the recovery check takes from crumbkit and not from the
//! tool: the codes in the tool's component that the tool does not declare,
//! of which only crumbkit's own codes and its add-on crates' codes have
//! their recoveries followed, and the warnings that crumbkit prints.

use crumbkit::Code;
use snapbox::assert::Action;

use super::{OTHERS, SHOW_7, Setup, check_with, failure, problems, recoveries, run};
use crate::recoveries::Failing;
use crate::recoveries::chain::known::ADD_ON_CODES;

/// Returns the setup of a chain whose step `tool books list` fails with
/// `code` and recovers by running `tool books show 2`, which succeeds.
fn failing_with(label: &str, code: &str) -> Setup {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let failed = failure(code, &run("tool books show 2"));
    Setup::new(
        label,
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, failed.as_str(), ""),
            ("books show 2 --json", 0, "{}\n", ""),
        ],
    )
}

#[test]
fn an_add_on_crates_code_spelled_with_the_tools_name_has_its_recovery_followed() {
    for (label, code) in [
        ("recoveries-add-on-child", "tool::child::not_found"),
        ("recoveries-add-on-aliases", "tool::aliases::unreadable"),
    ] {
        let setup = failing_with(label, code);

        check_with(
            &setup,
            &recoveries(&setup, SHOW_7, OTHERS),
            Action::Overwrite,
        );

        let report = setup.report();
        assert!(
            report.contains(&format!(
                "\x20   fails with {code}\n\
                 \x20   run tool books show 2\n\
                 \x20     exits 0\n"
            )),
            "{report}"
        );
    }
}

#[test]
fn a_code_of_crumbkit_spelled_with_the_tools_name_has_its_recovery_followed() {
    let setup = failing_with("recoveries-crumbkit-code", "tool::output::over_budget");

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
}

#[test]
fn an_undeclared_code_in_the_tools_component_that_no_crumbkit_crate_raises_fails_the_check() {
    for (label, code) in [
        ("recoveries-cache-stale", "tool::cache::stale"),
        ("recoveries-cli-bogus", "tool::cli::bogus"),
        ("recoveries-output-foo", "tool::output::foo"),
    ] {
        let setup = failing_with(label, code);

        let problems = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap_or_default();

        assert!(
            problems.contains(&format!(
                "tool books show 7: the step `tool books list` fails with {code}, \
                 whose component tool is a declared component, and no declared code spells it"
            )),
            "{code}: {problems}"
        );
    }
}

#[test]
fn an_undeclared_start_in_the_crumbkit_component_on_neither_built_in_list_fails_the_check() {
    const GONE: Code = Code::new("crumbkit", "books", "gone");
    let show = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let gone = failure("tool::books::gone", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-unlisted-start",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 9 --json", 1, gone.as_str(), ""),
        ],
    );
    let starts = [SHOW_7[0], Failing::new(&GONE, &["books", "show", "9"])];

    let message = problems(&setup, &recoveries(&setup, &starts, OTHERS));

    assert_eq!(
        message.as_deref(),
        Some(
            "the tool's recoveries break crumbkit's recovery rules:\n\
             tool books show 9: the start's code tool::books::gone is not a declared code"
        )
    );
}

#[test]
fn undeclared_starts_with_the_add_on_crates_codes_run_without_a_declaration() {
    let show = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let child = failure("tool::child::not_found", r#"{"kind":"retry"}"#);
    let aliases = failure("tool::aliases::unreadable", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-add-on-starts",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 8 --json", 1, child.as_str(), ""),
            ("books show 9 --json", 1, aliases.as_str(), ""),
        ],
    );
    let starts = [
        SHOW_7[0],
        Failing::new(&crumbkit_child::codes::NOT_FOUND, &["books", "show", "8"]),
        Failing::new(
            &crumbkit_aliases::codes::UNREADABLE,
            &["books", "show", "9"],
        ),
    ];

    assert_eq!(problems(&setup, &recoveries(&setup, &starts, OTHERS)), None);
}

#[test]
fn the_add_on_codes_are_the_codes_that_crumbkit_child_and_crumbkit_aliases_declare() {
    let declared: Vec<_> = crumbkit_child::codes::ALL
        .iter()
        .chain(crumbkit_aliases::codes::ALL)
        .copied()
        .copied()
        .collect();
    let listed: Vec<_> = ADD_ON_CODES.iter().copied().copied().collect();

    assert_eq!(listed, declared);
}

#[test]
fn a_step_that_writes_the_tools_own_warning_gives_no_problem() {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let setup = Setup::new(
        "recoveries-own-warning",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            (
                "books list --json",
                0,
                "{}\n",
                "warning: tool cache is stale\n",
            ),
        ],
    );

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
}
