// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`Failure`]: crate::Failure
//! [`Recovery`]: crate::Recovery
//! [`Code`]: crate::Code
//! [`Spelling`]: crate::Spelling
//! [`Error`]: crate::Error
//! [`Document`]: crate::Document
//! [`Call`]: crate::Call
//! [`Row`]: crate::Row
//! [`Listing`]: crate::Listing
//! [`Batch`]: crate::Batch
//! [crumbkit]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit/README.md
//! [ADR-0001]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0001-error-model.md
//! [ADR-0032]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0032-core-crate.md
#![doc = include_str!("../README.md")]

mod batch;
mod call;
mod code;
mod document;
mod failure;
mod listing;
#[cfg(feature = "miette")]
mod report;
mod row;
mod segment;

pub use crate::batch::Batch;
pub use crate::call::{Call, CallError, CallTarget};
pub use crate::code::{Code, Spelling};
pub use crate::document::Document;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::listing::{Listing, NoSummary};
pub use crate::row::{Row, Rows, Shape};
