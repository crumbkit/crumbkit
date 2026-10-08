//! Tests for shelf as a whole.

use crate::SPELLING;
use crate::books::BooksError;

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[BooksError::CODES]);
}
