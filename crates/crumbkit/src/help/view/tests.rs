//! Tests for the rule that decides which view flags a command offers.

use super::{Kind, Printed, Unoffered, ViewFlag, unoffered};

#[test]
fn a_stream_without_the_envelope_offers_fields_and_refuses_full() {
    let printed = Printed {
        kind: Kind::Stream,
        columns: &["id"],
        long: &[],
        envelope: false,
        headed: false,
        whole: false,
    };

    assert_eq!(unoffered(&printed, ViewFlag::Fields), None);
    assert_eq!(
        unoffered(&printed, ViewFlag::Full),
        Some(Unoffered::Unwrapped)
    );
}
