//! The `self figures list` command, which prints the figures of each
//! command in a tool's usage log (ADR-0029).
//!
//! The tool owns the command. It adds a `figures` resource with a `list`
//! verb that takes [`SelfFigures`] to its `self` group, and its handler
//! passes the result of [`figures`] to [`finish`](crate::finish) with the
//! document [`SELF_FIGURES_LIST`]:
//!
//! ```ignore
//! SelfCommand::Figures { verb: FiguresVerb::List(args) } => crumbkit::finish(
//!     &cli.output,
//!     SPELLING,
//!     crumbkit::usage::SELF_FIGURES_LIST,
//!     Ok::<_, crumbkit::Error>(crumbkit::usage::figures(&cli.output, &GUIDE, &args)),
//! ),
//! ```
//!
//! The listing prints one row for each command that the log names, in the
//! order of its path, with the summary line of `self usage` above the
//! table:
//!
//! ```text
//! 11 runs in 1 session since 4 Oct: person 11.
//! COMMAND     RUNS  HELP_PAGES  HELP_BEFORE  FAILED  P95_OUT_BYTES
//! shelf       0     1           0            0       -
//! books       0     1           0            0       -
//! books list  3     1           0            0       512
//! books show  5     0           3            2       86
//! ```
//!
//! `cargo crumbkit weigh --usage` runs it to join each command's figures to
//! the bytes of its help and output.

use std::io::BufRead;
use std::time::{Duration, SystemTime};

use clap::{Args, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::self_usage::{CommandFigures, SelfUsage, UsageSummary, open, summarize};
use crate::help::Guide;
use crate::{Document, Listing, Output, Page};

#[cfg(test)]
mod tests;

/// The document that `self figures list` prints: one row for each command
/// in the usage log, and the summary of the runs that it read. Its schema
/// is `crates/crumbkit/schemas/self-figures-list-v1.json`, which
/// [`schemas::document_schema`](crate::schemas::document_schema) builds.
pub const SELF_FIGURES_LIST: Document<Listing<FiguresRow, UsageSummary>> = Document::new(
    "crumbkit",
    "self-figures-list",
    1,
    "The figures of each command in the usage log, and a summary of the runs read.",
);

/// The arguments of `<tool> self figures list`: the arguments of
/// [`SelfUsage`], and how many rows to print.
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SelfFigures {
    /// How far back to read and which log to read.
    #[command(flatten)]
    usage: SelfUsage,

    /// How many commands to print.
    #[command(flatten)]
    page: Page,
}

/// The figures of one command in the usage log.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FiguresRow {
    /// The command path below the tool's name, such as `books show`, or the
    /// tool's name for its root command.
    command: String,
    /// The command's figures since the start of the listing.
    #[serde(flatten)]
    figures: CommandFigures,
}

impl crate::Row for FiguresRow {
    const COLUMNS: &'static [&'static str] = &[
        "command",
        "runs",
        "help_pages",
        "help_before",
        "failed",
        "p95_out_bytes",
    ];
    const LONG_COLUMNS: &'static [&'static str] =
        &["refused", "p95_ms", "typed_full", "typed_all", "typed_long"];
}

/// Returns the figures of each command in the usage log of the tool that
/// the tool passed to [`Output::set_command`], for the runs that ended in
/// the `--since` duration before now, one page of them as `args` asks, for
/// the tool to pass to [`finish`](crate::finish) with [`SELF_FIGURES_LIST`].
///
/// It reads the log that [`findings`](super::findings) reads, in the same
/// way, and leaves out and counts the same records. The rows follow the
/// order of the command paths, with the root first under the tool's name.
/// A command that the log does not name has no row. `guide` is the tool's
/// guide, whose exit codes give the meaning of each exit from 3.
#[must_use]
pub fn figures(
    output: &mut Output,
    guide: &Guide,
    args: &SelfFigures,
) -> Listing<FiguresRow, UsageSummary> {
    let empty = Command::new("");
    let tool = output.command().unwrap_or(&empty);
    figures_from(
        tool,
        guide,
        open(tool, &args.usage),
        args.usage.since,
        SystemTime::now(),
        args.page.limit(),
    )
}

/// Returns the figures in `log` for the runs that ended in the `since`
/// duration before `now`, checked against `tool`, with at most `limit`
/// rows, as [`figures`] describes for `guide`.
fn figures_from(
    tool: &Command,
    guide: &Guide,
    log: Option<impl BufRead>,
    since: Duration,
    now: SystemTime,
    limit: Option<usize>,
) -> Listing<FiguresRow, UsageSummary> {
    let (report, summary) = summarize(tool, guide, log, since, now);
    let rows = report
        .commands
        .iter()
        .map(|figures| FiguresRow {
            command: if figures.command.is_empty() {
                tool.get_name().to_owned()
            } else {
                figures.command.clone()
            },
            figures: CommandFigures::of(figures, guide),
        })
        .collect();
    Listing::from_all(rows, limit)
        .described("commands")
        .summary(summary)
}
