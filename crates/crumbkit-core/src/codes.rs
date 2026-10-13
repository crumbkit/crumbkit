//! The codes that crumbkit-core itself gives to a failure.
//!
//! crumbkit's own `codes` module re-exports each of them and lists them in
//! `codes::ALL`, so a tool's codes check sees them beside the tool's codes.

use crate::Code;

/// The code of a failure that the tool did not classify. An error passed to
/// [`Error::unclassified`](crate::Error::unclassified) has it, and so does a
/// `miette::Report` converted into an [`Error`](crate::Error) under the
/// `miette` feature.
pub const REPORT_UNCHECKED: Code = Code::new("crumbkit", "report", "unchecked");

/// The reason of the `Recovery::None` of an unclassified failure that gives
/// no advice of its own.
pub(crate) const NO_ADVICE: &str = "the error gives no advice";
