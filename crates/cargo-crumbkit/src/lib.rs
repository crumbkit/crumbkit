// The crate docs are the README. The README links to files in the
// repository, which work on GitHub. The definitions below come first, so
// rustdoc uses them for the same labels (ADR-0013).
//! [schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/schemas/weigh-v1.json
//! [shelf's weights]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
//! [ADR-0033]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0033-weigh.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod cargo;
mod cli;
mod error;
mod tool;
mod weigh;

use std::process::ExitCode;

use clap::CommandFactory as _;
use crumbkit::Spelling;

use crate::cli::{CargoCommand, CommandsCommand, CrumbkitCommand};
pub use crate::cli::{Cli, GUIDE};

#[cfg(test)]
mod tests;

/// How cargo-crumbkit spells its failure codes, such as
/// `crumbkit::weigh::build_failed`. The tests pass the same spelling to
/// crumbkit-test's codes check.
const SPELLING: Spelling = Spelling::COLONS;

/// Parses the process's command line, runs the command and returns the
/// process's exit code.
///
/// cargo runs `cargo crumbkit weigh` as `cargo-crumbkit crumbkit weigh`, so
/// the command tree's root is `cargo` and every command line that help,
/// `Next:` lines and recoveries print starts with `cargo crumbkit`.
#[must_use]
pub fn run() -> ExitCode {
    let (mut cli, command) = match crumbkit::parse::<Cli>(Cli::command(), &GUIDE, SPELLING) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.code(),
    };
    cli.output.set_command(command);
    match cli.cargo {
        CargoCommand::Crumbkit { command } => match command {
            CrumbkitCommand::Weigh(args) => {
                crumbkit::finish(&mut cli.output, SPELLING, weigh::WEIGH, weigh::weigh(&args))
            }
            CrumbkitCommand::Commands { command } => match command {
                // `commands list` reads the command tree through
                // `&mut cli.output`, so it runs before `finish`.
                CommandsCommand::List(args) => {
                    let result = crumbkit::commands::list(&mut cli.output, &GUIDE, &args);
                    let document = crumbkit::commands::COMMANDS_LIST;
                    crumbkit::finish(&mut cli.output, SPELLING, document, result)
                }
            },
        },
    }
}
