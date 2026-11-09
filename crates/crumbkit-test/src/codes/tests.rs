//! Tests for the codes check.

use crumbkit::Code;

use super::check_codes;

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");
const AUTHOR_NOT_FOUND: Code = Code::new("shelf", "authors", "not_found");

#[test]
fn distinct_codes_pass() {
    check_codes(&[&[&NOT_FOUND, &TIMED_OUT], &[&AUTHOR_NOT_FOUND]]);
}

#[test]
fn the_same_code_listed_by_two_error_types_passes() {
    check_codes(&[&[&NOT_FOUND], &[&NOT_FOUND, &TIMED_OUT]]);
}
