//! cargo-crumbkit's command-line grammar.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use crumbkit::Output;
use crumbkit::commands::{COMMANDS, CommandsList};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf};

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
/// group, so `cargo crumbkit -h` lists `weigh` under `Measure a tool's help`
/// and `commands` under `Explore the commands`. The help of
/// `cargo crumbkit weigh` and `cargo crumbkit commands list` lists their
/// examples, and their `--help` describes the columns of the rows they print.
pub const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(
        &["crumbkit"],
        &[
            Goal::new("Measure a tool's help", &["weigh"]),
            Goal::new("Explore the commands", &["commands"]),
        ],
    )])
    .leaves(&[
        Leaf::new(&["crumbkit", "weigh"])
            .examples(&[
                Example::new(&["--bin", "shelf"], "Weigh each path through shelf's help"),
                Example::new(&["--bin", "shelf", "--json"], "Print the weights as JSON"),
            ])
            .document(WEIGH),
        Leaf::new(&["crumbkit", "commands", "list"])
            .examples(&[Example::new(&[], "List every command")])
            .document(COMMANDS),
    ]);

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
#[derive(Debug, Args)]
pub(crate) struct WeighArgs {
    /// Build this binary of the current workspace and weigh its help
    #[arg(long, value_name = "NAME")]
    pub(crate) bin: String,

    /// Read the examples' golden files from this directory [default: tests/golden/examples in the binary's package]
    #[arg(long, value_name = "DIR")]
    pub(crate) goldens: Option<PathBuf>,
}
