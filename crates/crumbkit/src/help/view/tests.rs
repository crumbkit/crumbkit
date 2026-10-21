//! Tests for the rule that decides which view flags a command offers.

use super::{Kind, Listed, Printed, Unoffered, ViewFlag, unoffered};

#[test]
fn a_stream_without_the_envelope_offers_fields_and_refuses_full() {
    let printed = Printed {
        kind: Kind::Stream,
        columns: &["id"],
        long: &[],
        envelope: false,
        headed: false,
        whole: false,
        listed: Listed::Every,
        long_note: false,
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
        listed: Listed::Problems,
        long_note: false,
    };

    assert_eq!(unoffered(&printed, ViewFlag::Long), None);
    let every_row = Printed {
        listed: Listed::Every,
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
        listed: Listed::Every,
        long_note: true,
    };

    assert_eq!(unoffered(&printed, ViewFlag::Long), None);
    let unnoted = Printed {
        long_note: false,
        ..printed
    };
    assert_eq!(
        unoffered(&unnoted, ViewFlag::Long),
        Some(Unoffered::NoLongColumns)
    );
}
