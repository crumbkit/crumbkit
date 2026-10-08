//! Tests for `Error` and its conversions.

use std::fmt;

use super::{AsFailure as _, Error, Failure, Recovery};
use crate::Code;

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

/// A failure for a missing book.
#[derive(Debug)]
struct NoSuchBook(u32);

impl fmt::Display for NoSuchBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the catalogue has no book {}", self.0)
    }
}

impl std::error::Error for NoSuchBook {}

impl Failure for NoSuchBook {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("list the books first".to_owned())
    }
}

/// A failure for a request that took too long.
#[derive(Debug)]
struct Timeout;

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the catalogue did not answer in time")
    }
}

impl std::error::Error for Timeout {}

impl Failure for Timeout {
    fn code(&self) -> &'static Code {
        &TIMED_OUT
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

fn find(id: u32) -> Result<u32, NoSuchBook> {
    if id <= 6 { Ok(id) } else { Err(NoSuchBook(id)) }
}

fn fetch(online: bool) -> Result<(), Timeout> {
    if online { Ok(()) } else { Err(Timeout) }
}

/// Calls both functions with `?`, so each error type converts into `Error`.
fn lookup(id: u32, online: bool) -> Result<u32, Error> {
    fetch(online)?;
    let id = find(id)?;
    Ok(id)
}

#[test]
fn question_mark_converts_two_different_failure_types_into_an_error() {
    let missing = lookup(7, true).unwrap_err();
    let slow = lookup(2, false).unwrap_err();

    assert_eq!(missing.failure().code(), &NOT_FOUND);
    assert_eq!(slow.failure().code(), &TIMED_OUT);
    assert_eq!(slow.failure().recovery(), Recovery::Retry);
    assert_eq!(lookup(2, true).unwrap(), 2);
}

#[test]
fn an_error_prints_the_display_and_debug_of_the_failure_inside() {
    let error = Error::from(NoSuchBook(7));

    assert_eq!(error.to_string(), "the catalogue has no book 7");
    assert_eq!(format!("{error:?}"), "NoSuchBook(7)");
}

#[test]
fn the_failure_inside_an_error_downcasts_to_its_own_type() {
    let error = Error::from(NoSuchBook(7));

    let failure: &(dyn std::error::Error + 'static) = error.failure();

    assert_eq!(failure.downcast_ref::<NoSuchBook>().unwrap().0, 7);
}

#[test]
fn a_failure_and_an_error_both_read_as_the_same_failure() {
    let failure = NoSuchBook(7);
    let error = Error::from(NoSuchBook(7));

    assert_eq!(failure.as_failure().code(), error.as_failure().code());
    assert_eq!(
        failure.as_failure().to_string(),
        error.as_failure().to_string()
    );
}
