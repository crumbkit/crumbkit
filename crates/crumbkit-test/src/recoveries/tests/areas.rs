//! Tests for `Unprovoked::area`, which lists every declared code of one
//! area under one reason.

use crumbkit::Code;

use super::{NOT_FOUND, REMOVED, SHOW_7, Setup, UNREADABLE, failure, problems, recoveries};
use crate::examples::Examples;
use crate::recoveries::{Recoveries, Unprovoked};

/// The area of every declared code.
const BOOKS: Unprovoked<'_> = Unprovoked::area("tool", "books", "Only a server provokes them.");

/// Returns a setup whose script fails book 7 with `NOT_FOUND`.
fn setup(label: &str) -> Setup {
    let show = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    Setup::new(label, &[("books show 7 --json", 1, show.as_str(), "")])
}

#[test]
fn an_area_entry_excuses_every_declared_code_of_its_area_that_no_start_provokes() {
    let setup = setup("recoveries-area-excuses");
    let codes = [
        NOT_FOUND,
        UNREADABLE,
        REMOVED,
        Code::from_spelling("tool::api::down"),
    ];

    let message = problems(
        &setup,
        &Recoveries::new(Examples::new(&setup.script, ""), &codes)
            .starts(SHOW_7)
            .unprovoked(&[
                Unprovoked::area("tool", "books", "Only a server provokes them."),
                Unprovoked::area("tool", "api", "The binary never prints it."),
            ]),
    );

    assert_eq!(message, None);
}

#[test]
fn an_area_entry_that_covers_no_declared_code_fails_the_recovery_check() {
    let setup = setup("recoveries-area-typo");
    let unprovoked = [BOOKS, Unprovoked::area("tool", "bokos", "A misspelt area.")];

    let message = problems(&setup, &recoveries(&setup, SHOW_7, &unprovoked)).unwrap();

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool::bokos::*: Recoveries::unprovoked lists the area, and it covers no declared code"
    );
}

#[test]
fn an_area_entry_whose_every_code_a_start_provokes_fails_the_recovery_check_as_stale() {
    let setup = setup("recoveries-area-stale");
    let codes = [NOT_FOUND];
    let unprovoked = [BOOKS];

    let message = problems(
        &setup,
        &Recoveries::new(Examples::new(&setup.script, ""), &codes)
            .starts(SHOW_7)
            .unprovoked(&unprovoked),
    )
    .unwrap();

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool::books::*: Recoveries::unprovoked lists the area, and a start provokes every declared code in it"
    );
}

#[test]
fn a_code_entry_inside_a_listed_area_fails_the_recovery_check() {
    let setup = setup("recoveries-area-repeat");
    let unprovoked = [
        BOOKS,
        Unprovoked::new(REMOVED, "Only a full disk provokes it."),
    ];

    let message = problems(&setup, &recoveries(&setup, SHOW_7, &unprovoked)).unwrap();

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool::books::removed_file: Recoveries::unprovoked lists the code, and it repeats the area entry tool::books::*"
    );
}

#[test]
fn the_recovery_report_lists_an_area_entry_with_each_code_it_excuses() {
    let setup = setup("recoveries-area-report");
    let unprovoked = [BOOKS];

    let message = problems(&setup, &recoveries(&setup, SHOW_7, &unprovoked));

    assert_eq!(message, None);
    let report = setup.report();
    assert!(
        report.contains(
            "codes that no start provokes:\n\
             \x20 tool::books::*: Only a server provokes them.\n\
             \x20   tool::books::unreadable\n\
             \x20   tool::books::removed_file\n"
        ),
        "{report}"
    );
}
