//! Tests for the rule that decides which view flags a command offers.

use super::{Kind, Printed, Unoffered, ViewFlag, adds_nothing, adds_nothing_note, unoffered};
use crate::document::ListingText;

/// A listing's text that does nothing beyond its columns.
const PLAIN: ListingText = ListingText {
    only_problems: false,
    long_note: false,
};

#[test]
fn a_stream_without_the_envelope_offers_fields_and_refuses_full() {
    let printed = Printed {
        kind: Kind::Stream,
        columns: &["id"],
        long: &[],
        envelope: false,
        headed: false,
        whole: false,
        listing: PLAIN,
    };

    assert_eq!(unoffered(&printed, ViewFlag::Fields), None);
    assert_eq!(
        unoffered(&printed, ViewFlag::Full),
        Some(Unoffered::Unwrapped)
    );
}

#[test]
fn a_document_that_lists_only_problems_offers_long_without_long_columns() {
    let printed = Printed {
        kind: Kind::Document(crate::Shape::Listing),
        columns: &["id"],
        long: &[],
        envelope: true,
        headed: false,
        whole: false,
        listing: ListingText {
            only_problems: true,
            ..PLAIN
        },
    };

    assert_eq!(unoffered(&printed, ViewFlag::Long), None);
    let every_row = Printed {
        listing: PLAIN,
        ..printed
    };
    assert_eq!(
        unoffered(&every_row, ViewFlag::Long),
        Some(Unoffered::NoLongColumns)
    );
}

#[test]
fn a_document_that_declares_a_long_note_offers_long_without_long_columns() {
    let printed = Printed {
        kind: Kind::Document(crate::Shape::Listing),
        columns: &["id"],
        long: &[],
        envelope: true,
        headed: false,
        whole: false,
        listing: ListingText {
            long_note: true,
            ..PLAIN
        },
    };

    assert_eq!(unoffered(&printed, ViewFlag::Long), None);
    let unnoted = Printed {
        listing: PLAIN,
        ..printed
    };
    assert_eq!(
        unoffered(&unnoted, ViewFlag::Long),
        Some(Unoffered::NoLongColumns)
    );
}

/// Returns what a listing of whole rows with one column and no long
/// columns prints.
const fn whole_listing() -> Printed {
    Printed {
        kind: Kind::Document(crate::Shape::Listing),
        columns: &["id"],
        long: &[],
        envelope: true,
        headed: false,
        whole: true,
        listing: PLAIN,
    }
}

#[test]
fn long_and_full_add_nothing_to_the_json_of_whole_rows() {
    let printed = whole_listing();

    assert!(adds_nothing(&printed, ViewFlag::Long, true));
    assert!(adds_nothing(&printed, ViewFlag::Full, true));
    assert!(!adds_nothing(&printed, ViewFlag::Fields, true));
}

#[test]
fn long_and_full_without_json_are_not_accepted_as_adding_nothing() {
    let printed = whole_listing();

    assert!(!adds_nothing(&printed, ViewFlag::Long, false));
    assert!(!adds_nothing(&printed, ViewFlag::Full, false));
}

#[test]
fn long_and_full_with_json_on_rows_that_do_not_print_whole_are_not_accepted_as_adding_nothing() {
    let printed = Printed {
        whole: false,
        ..whole_listing()
    };

    assert!(!adds_nothing(&printed, ViewFlag::Long, true));
    assert!(!adds_nothing(&printed, ViewFlag::Full, true));
}

#[test]
fn the_note_names_the_flag_that_adds_nothing() {
    assert_eq!(
        adds_nothing_note(ViewFlag::Long),
        "note: --long adds nothing to --json here; the JSON already holds the whole result."
    );
    assert_eq!(
        adds_nothing_note(ViewFlag::Full),
        "note: --full adds nothing to --json here; the JSON already holds the whole result."
    );
}
