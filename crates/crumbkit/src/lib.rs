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
//! [`trace`]: crate::trace
//! [golden files]: https://github.com/crumbkit/crumbkit/tree/main/examples/shelf/tests/golden
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit/schemas
//! [crumbkit-test]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-test/README.md
//! [crumbkit-core]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-core/README.md
//! [ADR-0001]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0001-error-model.md
//! [ADR-0007]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0007-json-schemas.md
//! [ADR-0016]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/output/0016-result-formats.md
//! [ADR-0021]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0021-help-pages.md
//! [ADR-0023]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0023-help-command-and-topics.md
//! [ADR-0024]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0024-leaf-examples-and-columns.md
//! [ADR-0026]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0026-branch-export.md
//! [ADR-0029]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0029-usage-log.md
//! [ADR-0030]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0030-trace-context.md
//! [ADR-0032]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0032-core-crate.md
#![doc = include_str!("../README.md")]

mod call;
pub mod commands;
#[cfg(any(feature = "usage-log", feature = "tracing"))]
mod exit;
mod fetch;
pub mod help;
mod output;
mod page;
mod parse;
mod refusal;
mod row;
mod sink;
pub mod trace;
pub mod usage;

// crumbkit re-exports every item of crumbkit-core at the path it had in
// crumbkit, so a tool depends on crumbkit alone (ADR-0032).
pub use crumbkit_core::{
    AsFailure, Batch, Call, CallError, CallTarget, Code, Document, Error, Failure, Listing,
    Recovery, Row, Rows, Spelling,
};

pub use crate::call::RenderCall;
pub use crate::fetch::fetch_each;
pub use crate::output::{Output, finish, finish_to};
pub use crate::page::Page;
pub use crate::parse::{parse, parse_from, parse_to};
pub use crate::sink::Sink;
