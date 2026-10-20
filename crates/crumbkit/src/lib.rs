// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`Output`]: crate::Output
//! [`finish`]: crate::finish
//! [`Row`]: crate::Row
//! [`Document`]: crate::Document
//! [`Listing`]: crate::Listing
//! [`Page`]: crate::Page
//! [`Batch`]: crate::Batch
//! [`Prose`]: crate::Prose
//! [`Failure`]: crate::Failure
//! [`Recovery`]: crate::Recovery
//! [`Code`]: crate::Code
//! [`Spelling`]: crate::Spelling
//! [`Error`]: crate::Error
//! [`parse`]: crate::parse()
//! [`help`]: crate::help
//! [`Call`]: crate::Call
//! [`commands`]: crate::commands
//! [`fetch_each`]: crate::fetch_each
//! [`events`]: crate::events
//! [crumbkit-usage]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-usage/README.md
//! [`schemas`]: crate::schemas
//! [`trace`]: crate::trace
//! [tutorial]: https://github.com/crumbkit/crumbkit/blob/main/docs/tutorial.md
//! [golden files]: https://github.com/crumbkit/crumbkit/tree/main/examples/shelf/tests/golden
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit/schemas
//! [crumbkit-test]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/README.md
//! [crumbkit-derive]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-derive/README.md
//! [behaviour docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/README.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod age;
mod agent;
#[cfg(test)]
mod answered;
mod batch;
mod bytes;
mod call;
mod code;
pub mod codes;
pub mod commands;
mod complete;
mod dashed;
mod document;
pub mod events;
mod exit;
mod expect;
mod failure;
mod fetch;
mod field;
pub mod form;
pub mod help;
mod listing;
mod output;
mod page;
mod parse;
pub mod pick;
mod prose;
mod refusal;
#[cfg(doctest)]
mod removed;
#[cfg(feature = "miette")]
mod report;
mod row;
pub mod schemas;
mod segment;
mod sink;
mod stream;
mod tool_name;
pub mod trace;
#[cfg(test)]
mod twins;
mod unclassified;
mod view;

pub use crate::age::Age;
pub use crate::batch::Batch;
pub use crate::bytes::Bytes;
pub use crate::call::{Call, CallArg, CallError, CallTarget, RenderCall};
pub use crate::code::{Code, RuntimeCodeError, Spelling};
#[cfg(feature = "complete")]
pub use crate::complete::{complete, complete_to, complete_with, complete_with_words};
pub use crate::dashed::{dashed_form, dashed_name, is_dashed};
pub use crate::document::{Document, Envelope};
pub use crate::expect::Expect;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::fetch::fetch_each;
pub use crate::field::{Field, FieldValue};
pub use crate::listing::{Listing, NoSummary};
pub use crate::output::{
    DryRun, JsonLayout, Output, Room, Streaming, ViewFlags, Yes, finish, finish_to, stream,
    stream_to,
};
pub use crate::page::Page;
pub use crate::parse::walk::command_path;
pub use crate::parse::{
    Printed, Refused, Stop, Unparsed, parse, parse_from, parse_to, try_parse, try_parse_from,
    try_parse_to,
};
pub use crate::pick::NoInput;
pub use crate::prose::Prose;
pub use crate::row::{Headed, NoRow, Row, Rows, Shape};
pub use crate::sink::Sink;
pub use crate::stream::Stream;
pub use crate::tool_name::ToolName;
/// Writes the [`Failure`] impl and the `CODES` list of an enum or a struct
/// from `#[failure(...)]` attributes. The `derive` feature adds it, and
/// crumbkit-derive documents its attributes.
#[cfg(feature = "derive")]
pub use crumbkit_derive::Failure;

/// The schemars crate that crumbkit builds against, so a tool can name
/// `crumbkit::schemars::{Schema, json_schema}` without a schemars
/// dependency of its own.
///
/// The `JsonSchema` bound on a result already makes schemars' major version
/// part of crumbkit's API, so a new major version of schemars is a breaking
/// release of crumbkit with or without this re-export.
///
/// crumbkit turns on no schemars feature in its default build, so
/// `#[derive(JsonSchema)]` through this path works only when some crate in
/// the build turns on schemars' `derive` feature, and it then needs
/// `#[schemars(crate = "crumbkit::schemars")]`. A tool that derives
/// `JsonSchema` for its results keeps its own schemars dependency with
/// `derive`, as the README shows.
pub use schemars;
