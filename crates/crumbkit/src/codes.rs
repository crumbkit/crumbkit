//! crumbkit's own failure codes, which a tool's tests name when they expect
//! a run to fail with one of them.
//!
//! Each constant is the code that crumbkit returns for one of its own
//! failures, and [`ALL`] lists them, so crumbkit-test's recovery check
//! follows a chain through a crumbkit failure without asking the tool to
//! declare its codes ([ADR-0001]). A tool still spells each code with its own
//! [`Spelling`](crate::Spelling), such as `<tool>::cli::usage` in the colon
//! spelling.
//!
//! ```
//! use crumbkit::Spelling;
//! use crumbkit::codes::CLI_USAGE;
//!
//! assert_eq!(Spelling::COLONS.spell(&CLI_USAGE), "crumbkit::cli::usage");
//! ```
//!
//! [ADR-0001]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/failures/0001-error-model.md

use crate::Code;

#[cfg(test)]
mod tests;

/// The code of every refusal: a command line that clap refused, or a flag
/// value that crumbkit refused, such as an unknown `--fields` name. The run
/// exits with code 2 unless the tool's guide lists another exit for the
/// code, and a tool may print its own code in its place with
/// [`Rename`](crate::help::Rename).
pub const CLI_USAGE: Code = Code::new("crumbkit", "cli", "usage");

/// The code of a `commands list` `PATH` that names no visible command
/// (ADR-0026).
pub const COMMANDS_NOT_FOUND: Code = Code::new("crumbkit", "commands", "not_found");

/// The code of a `schemas show` id that names no document the tool prints
/// (ADR-0035).
pub const SCHEMAS_NOT_FOUND: Code = Code::new("crumbkit", "schemas", "not_found");

/// The code of a question that the person at the terminal did not answer
/// with yes (ADR-0039).
pub const CONFIRM_DECLINED: Code = Code::new("crumbkit", "confirmation", "declined");

/// The code of a question that no person can answer, because standard input
/// or standard error is not a terminal (ADR-0039).
pub const CONFIRM_NEEDS_YES: Code = Code::new("crumbkit", "confirmation", "needs_yes");

/// The code of a write whose target is not at the version that `--expect`
/// names, so the command changed nothing (ADR-0047).
pub const VERSION_CONFLICT: Code = Code::new("crumbkit", "version", "conflict");

/// The code of a result that does not fit in the output budget: a single
/// row or a prose result that crumbkit refused, the ids of a batch that it
/// left out, a stream that it stopped, and the mark of a listing that it cut
/// short. A document can set its own code in place of this one (ADR-0046).
pub const OUTPUT_OVER_BUDGET: Code = Code::new("crumbkit", "output", "over_budget");

/// The code of a failure that the tool did not classify: an error passed to
/// [`Error::unclassified`](crate::Error::unclassified), and a
/// `miette::Report` converted into an [`Error`](crate::Error) under the
/// `miette` feature (ADR-0001).
pub const REPORT_UNCHECKED: Code = Code::new("crumbkit", "report", "unchecked");

/// The reason of the `Recovery::None` of an unclassified failure that gives
/// no advice of its own, which `Recovery::no_advice` builds.
pub(crate) const NO_ADVICE: &str = "the error gives no advice";

/// Every code above, in the order this module declares them.
pub const ALL: &[&Code] = &[
    &CLI_USAGE,
    &COMMANDS_NOT_FOUND,
    &SCHEMAS_NOT_FOUND,
    &CONFIRM_DECLINED,
    &CONFIRM_NEEDS_YES,
    &VERSION_CONFLICT,
    &OUTPUT_OVER_BUDGET,
    &REPORT_UNCHECKED,
];
