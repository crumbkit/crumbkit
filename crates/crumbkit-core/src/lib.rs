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
//! [`Prose`]: crate::Prose
//! [crumbkit]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit/README.md
//! [ADR-0001]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0001-error-model.md
//! [ADR-0032]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0032-core-crate.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod batch;
mod call;
mod code;
mod document;
mod failure;
mod listing;
mod prose;
#[cfg(feature = "miette")]
mod report;
mod row;
mod segment;
mod stream;

pub use crate::batch::Batch;
pub use crate::call::{Call, CallArg, CallError, CallTarget};
pub use crate::code::{Code, Spelling};
pub use crate::document::Document;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::listing::{Listing, NoSummary};
pub use crate::prose::Prose;
pub use crate::row::{NoRow, Row, Rows, Shape};
pub use crate::stream::Stream;
