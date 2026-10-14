// The crate docs are the README. The README links to files in the
// repository, which work on GitHub. The definitions below come first, so
// rustdoc uses them for the same labels (ADR-0013).
//! [schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/schemas/weigh-v1.json
//! [shelf's weights]: https://github.com/crumbkit/crumbkit/blob/main/crates/cargo-crumbkit/tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
//! [ADR-0033]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0033-weigh.md
//! [shelf's reference]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/docs/commands.md
//! [ADR-0044]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0044-command-reference.md
//! [weigh docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/weight.md
//! [command reference docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/reference.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod cargo;
mod cli;
mod docs;
mod error;
mod tool;
mod weigh;

use std::process::ExitCode;

use clap::CommandFactory as _;
use crumbkit::Spelling;

pub use crate::cli::{ALLOW, Cli, GUIDE};
use crate::cli::{CargoCommand, CommandsCommand, CrumbkitCommand};

#[cfg(test)]
mod tests;

/// How cargo-crumbkit spells its failure codes, such as
/// `crumbkit::tool::build_failed`. The tests pass the same spelling to
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
    let (mut cli, parsed) = match crumbkit::parse::<Cli>(Cli::command(), &GUIDE, SPELLING) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.code(),
    };
    cli.output.set_command(parsed);
    match cli.cargo {
        CargoCommand::Crumbkit { command } => match command {
            CrumbkitCommand::Weigh(args) => {
                crumbkit::finish(&mut cli.output, SPELLING, weigh::WEIGH, weigh::weigh(&args))
            }
            CrumbkitCommand::Docs(args) => {
                crumbkit::finish(&mut cli.output, SPELLING, docs::DOCS, docs::docs(&args))
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
