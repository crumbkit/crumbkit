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
//! [ADR-0032]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0032-core-crate.md
//! [failure docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/failures/README.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod batch;
mod bytes;
mod call;
mod code;
pub mod codes;
mod document;
mod failure;
mod field;
mod listing;
mod prose;
#[cfg(feature = "miette")]
mod report;
mod row;
mod segment;
mod stream;
mod unclassified;

pub use crate::batch::Batch;
pub use crate::bytes::Bytes;
pub use crate::call::{Call, CallArg, CallError, CallTarget};
pub use crate::code::{Code, RuntimeCodeError, Spelling};
pub use crate::document::Document;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::field::{Field, FieldValue};
pub use crate::listing::{Listing, NoSummary};
pub use crate::prose::Prose;
pub use crate::row::{NoRow, Row, Rows, Shape};
pub use crate::stream::Stream;
/// Writes the [`Failure`] impl and the `CODES` list of an enum or a struct
/// from `#[failure(...)]` attributes. The `derive` feature adds it, and
/// crumbkit-derive documents its attributes.
#[cfg(feature = "derive")]
pub use crumbkit_derive::Failure;
