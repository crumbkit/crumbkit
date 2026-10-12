//! cargo-crumbkit's command-line grammar.

use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};
use crumbkit::Output;
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Effects, Example, Goal, Group, Guide, Leaf};
use crumbkit_test::{Allow, Rule};

use crate::docs::DOCS;
use crate::weigh::WEIGH;

#[cfg(test)]
mod tests;

/// Cargo subcommands for tools built with crumbkit.
// cargo runs the binary `cargo-crumbkit` with the words `crumbkit weigh`, so
// the root is named `cargo` and `crumbkit` is its one subcommand. Help and
// every printed command line then start with `cargo crumbkit`, which is what
// a person types.
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub struct Cli {
    /// The cargo subcommand, which is always `crumbkit`.
    #[command(subcommand)]
    pub(crate) cargo: CargoCommand,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) output: Output,
}

/// cargo-crumbkit's goal headings and leaves. It guides the `crumbkit`
/// group, so `cargo crumbkit -h` lists `weigh` under `Measure a tool's help`,
/// `docs` under `Write a tool's reference` and `commands` under
/// `Explore the commands`. The help of `cargo crumbkit weigh`,
/// `cargo crumbkit docs` and `cargo crumbkit commands list` lists their
/// examples, and their `--help` describes the columns of the rows they print.
/// Its root is `crumbkit`, because cargo always passes that word, so
/// `cargo crumbkit help` lists the topics.
pub const GUIDE: Guide = Guide::new()
    .root(&["crumbkit"])
    .groups(&[Group::new(
        &["crumbkit"],
        &[
            Goal::new("Measure a tool's help", &["weigh"]),
            Goal::new("Write a tool's reference", &["docs"]),
            Goal::new("Explore the commands", &["commands"]),
        ],
    )])
    .leaves(&[
        Leaf::new(&["crumbkit", "weigh"])
            .examples(&[
                Example::new(&["--bin", "shelf"], "Weigh each path through shelf's help"),
                Example::new(&["--bin", "shelf", "--json"], "Print the weights as JSON"),
                Example::new(
                    &["--bin", "shelf", "--usage"],
                    "Add the figures of shelf's own usage log",
                ),
            ])
            .document(WEIGH),
        Leaf::new(&["crumbkit", "docs"])
            .examples(&[Example::new(
                &["--bin", "shelf"],
                "Write shelf's reference to its docs/commands.md",
            )])
            .effects(Effects::additive().idempotent())
            .document(DOCS),
        Leaf::new(&["crumbkit", "commands", "list"])
            .examples(&[Example::new(&[], "List every command")])
            .document(COMMANDS_LIST),
    ]);

/// cargo-crumbkit's opt-outs from crumbkit's conventions, which its tests
/// pass to every check that takes them.
pub const ALLOW: &[Allow] = &[Allow::new(
    Rule::Examples,
    "crumbkit docs",
    "Each run of the example writes shelf's reference into the workspace, so tests/docs.rs runs docs with a file in a temporary directory instead.",
)];

/// The cargo subcommands that cargo-crumbkit serves.
#[derive(Debug, Subcommand)]
pub(crate) enum CargoCommand {
    /// Measure and explore tools built with crumbkit.
    #[command(version)]
    Crumbkit {
        /// What to do.
        #[command(subcommand)]
        command: CrumbkitCommand,
    },
}

/// The commands of `cargo crumbkit`.
#[derive(Debug, Subcommand)]
pub(crate) enum CrumbkitCommand {
    /// Measure the bytes that an agent reads on each path through a tool's help.
    Weigh(WeighArgs),
    /// Write a Markdown reference of every command of a tool to a file.
    Docs(DocsArgs),
    /// Explore cargo crumbkit's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
}

/// The commands for cargo crumbkit's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of cargo crumbkit's commands with their arguments, examples and columns.
    List(CommandsList),
}

/// The arguments of `cargo crumbkit weigh`.
// `--since` requires the group of `--usage` and `--log`, so it needs one of
// them, and `--log` alone turns the figures on.
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("figures").args(["usage", "log"]).multiple(true)))]
pub(crate) struct WeighArgs {
    /// Build this binary of the current workspace and weigh its help
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Read the examples' golden files from this directory [default: tests/golden/examples in the binary's package]
    #[arg(long, value_name = "DIR")]
    pub(crate) goldens: Option<PathBuf>,

    /// Add the runs, help pages and output of each command from the tool's usage log
    #[arg(long)]
    usage: bool,

    /// Read the usage figures from this log instead of the tool's own; implies --usage
    #[arg(long, value_name = "FILE")]
    pub(crate) log: Option<PathBuf>,

    /// How far back to read the usage log, such as 12h, 7d or 2w [default: 7d]
    #[arg(long, value_name = "DURATION", requires = "figures")]
    pub(crate) since: Option<String>,
}

impl WeighArgs {
    /// Reports whether `weigh` adds the usage figures, which `--usage` or
    /// `--log` asks for.
    pub(crate) const fn usage(&self) -> bool {
        self.usage || self.log.is_some()
    }
}

/// The arguments of `cargo crumbkit docs`.
#[derive(Debug, Args)]
pub(crate) struct DocsArgs {
    /// Build this binary of the current workspace and describe its commands
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Write the reference to this file [default: docs/commands.md in the binary's package]
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
}
