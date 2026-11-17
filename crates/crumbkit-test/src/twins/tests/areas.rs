//! Tests for `Unprovoked::area` in the twin check.

use super::coverage::{LIMIT_0, REMOVED, SHOW_7, checked, checked_allowing};
use crate::allow::Allow;
use crate::recoveries::{Failing, Unprovoked};

/// The area of every declared code.
const BOOKS: Unprovoked<'_> = Unprovoked::area("tool", "books", "Only a server provokes them.");

#[test]
fn an_area_entry_excuses_every_declared_code_of_its_area_that_no_twin_refuses() {
    assert_eq!(checked(&[SHOW_7, LIMIT_0], &[BOOKS]), None);
}

#[test]
fn an_area_entry_that_covers_no_declared_code_fails_the_twin_check() {
    let unprovoked = [BOOKS, Unprovoked::area("tool", "bokos", "A misspelt area.")];

    assert_eq!(
        checked(&[SHOW_7], &unprovoked).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::bokos::*: Twins::unprovoked lists the area, and it covers no declared code"
        )
    );
}

#[test]
fn an_untwinned_opt_out_for_a_code_in_a_listed_area_is_stale() {
    let allow = [Allow::untwinned(
        Failing::new(REMOVED, &["books", "show", "9"]),
        "Only a broken file provokes it.",
    )];

    let message = checked_allowing(&[SHOW_7], &[BOOKS], &allow).unwrap();

    assert!(
        message.contains(
            "names tool::books::removed_file, which Twins::unprovoked lists, so it is stale"
        ),
        "{message}"
    );
}
