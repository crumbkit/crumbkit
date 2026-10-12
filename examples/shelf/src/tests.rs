//! Tests for shelf as a whole that read its private items. The convention
//! tests, which need only shelf's public command tree and guide, live in
//! `tests/conventions.rs`.

use crate::SPELLING;
use crate::books::BooksError;

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[BooksError::CODES]);
}
