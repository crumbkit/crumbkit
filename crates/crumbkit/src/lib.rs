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
//! [`usage`]: crate::usage
//! [`schemas`]: crate::schemas
//! [`trace`]: crate::trace
//! [golden files]: https://github.com/crumbkit/crumbkit/tree/main/examples/shelf/tests/golden
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit/schemas
//! [crumbkit-test]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/README.md
//! [crumbkit-core]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-core/README.md
//! [ADR-0001]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/failures/0001-error-model.md
//! [ADR-0007]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/schemas/0007-json-schemas.md
//! [ADR-0016]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/output/results/0016-result-formats.md
//! [ADR-0021]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0021-help-pages.md
//! [ADR-0023]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0023-help-command-and-topics.md
//! [ADR-0024]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0024-leaf-examples-and-columns.md
//! [ADR-0026]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0026-branch-export.md
//! [ADR-0029]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0029-usage-log.md
//! [ADR-0030]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0030-trace-context.md
//! [ADR-0032]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0032-core-crate.md
//! [ADR-0048]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0048-completion.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod agent;
mod call;
#[cfg(feature = "hand-over")]
pub mod child;
pub mod codes;
pub mod commands;
#[cfg(any(feature = "complete", feature = "usage-log"))]
mod complete;
mod dashed;
mod exit;
mod expect;
mod fetch;
pub mod form;
pub mod help;
mod output;
mod page;
mod parse;
pub mod pick;
mod refusal;
mod row;
pub mod schemas;
mod sink;
pub mod trace;
#[cfg(test)]
mod twins;
pub mod usage;

// crumbkit re-exports every item of crumbkit-core at the path it had in
// crumbkit, so a tool depends on crumbkit alone (ADR-0032).
pub use crumbkit_core::{
    AsFailure, Batch, Bytes, Call, CallArg, CallError, CallTarget, Code, Document, Error, Failure,
    Field, FieldValue, Listing, NoRow, NoSummary, Prose, Recovery, Row, Rows, RuntimeCodeError,
    Shape, Spelling, Stream,
};

pub use crate::call::RenderCall;
#[cfg(feature = "complete")]
pub use crate::complete::{complete, complete_to, complete_with, complete_with_words};
pub use crate::dashed::{dashed_form, dashed_name};
pub use crate::expect::{Expect, Expected};
pub use crate::fetch::fetch_each;
pub use crate::output::{
    DryRun, JsonLayout, Output, Streaming, Yes, finish, finish_to, stream, stream_to,
};
pub use crate::page::Page;
pub use crate::parse::walk::command_path;
pub use crate::parse::{Parsed, Stop, parse, parse_from, parse_to};
pub use crate::pick::NoInput;
pub use crate::sink::Sink;

/// The schemars crate that crumbkit builds against, so a tool can write a
/// [`FailureForm`](crate::form::FailureForm)'s schema with
/// `crumbkit::schemars::{Schema, json_schema}` and needs no schemars
/// dependency of its own (ADR-0045).
///
/// `FailureForm::schema` and the `JsonSchema` bound on a result already
/// make schemars' major version part of crumbkit's API, so a new major
/// version of schemars is a breaking release of crumbkit with or without
/// this re-export.
///
/// crumbkit turns on no schemars feature in its default build, so
/// `#[derive(JsonSchema)]` through this path works only when some crate in
/// the build turns on schemars' `derive` feature, and it then needs
/// `#[schemars(crate = "crumbkit::schemars")]`. A tool that derives
/// `JsonSchema` for its results keeps its own schemars dependency with
/// `derive`, as the README shows.
pub use schemars;
