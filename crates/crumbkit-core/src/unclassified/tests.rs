use std::fmt;
use std::io;

use crate::codes::REPORT_UNCHECKED;
use crate::{Error, Recovery};

/// An error that adds context to the error it wraps, as an error library's
/// context layer does.
#[derive(Debug)]
struct Context {
    message: &'static str,
    cause: io::Error,
}

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Context {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

#[test]
fn an_unclassified_error_has_the_unchecked_code_and_no_next_step() {
    let error = Error::unclassified(io::Error::other("disk full"));
    let failure = error.failure();
    assert_eq!(failure.code(), &REPORT_UNCHECKED);
    assert_eq!(
        failure.recovery(),
        Recovery::None {
            reason: "the error gives no advice".to_owned()
        }
    );
    assert_eq!(failure.alternatives(), Vec::new());
    assert_eq!(failure.fields(), Vec::new());
    assert_eq!(failure.url(), None);
}

#[test]
fn an_unclassified_error_prints_its_display_and_its_source_chain() {
    let error = Error::unclassified(Context {
        message: "cannot save the catalogue",
        cause: io::Error::other("disk full"),
    });
    assert_eq!(error.to_string(), "cannot save the catalogue");
    let cause = std::error::Error::source(&error).map(ToString::to_string);
    assert_eq!(cause.as_deref(), Some("disk full"));
}

#[test]
fn a_string_converts_into_an_unclassified_error_with_the_string_as_its_message() {
    let error = Error::unclassified("the catalogue is locked");
    assert_eq!(error.to_string(), "the catalogue is locked");
    assert_eq!(error.failure().code(), &REPORT_UNCHECKED);
}
