//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.
//!
//! The library holds the whole tool, and the `shelf` binary calls [`run`].
//! It exports the parser and the guide, so shelf's integration tests can
//! pass them to crumbkit-test's checks beside the built binary.

mod books;
mod cli;

use std::process::ExitCode;

use clap::CommandFactory as _;
use crumbkit::Spelling;

use crate::cli::{
    AliasesCommand, BooksCommand, CommandsCommand, FiguresCommand, Resource, SchemasCommand,
    SelfCommand,
};
pub use crate::cli::{Cli, GUIDE};

#[cfg(test)]
mod tests;

/// How shelf spells its failure codes, such as `shelf::books::not_found`.
/// shelf's test passes the same spelling to crumbkit-test's codes check.
const SPELLING: Spelling = Spelling::COLONS;

/// Parses the process's command line, runs the command and returns the
/// process's exit code. `crumbkit::parse_from` takes shelf's guide, which gives
/// its help goal headings and help topics. It prints help, the version or a
/// refused command line and returns the exit code. `crumbkit::finish` prints
/// the result or the failure in the format that the output flags choose and
/// returns the process's exit code.
///
/// Before parsing, `crumbkit_aliases::expand` replaces a personal alias in
/// the first word with its words from shelf's aliases file, so clap,
/// help and every output see only real commands.
///
/// shelf passes `set_command` the command that `crumbkit::parse_from`
/// returns, which holds crumbkit's help lines, and `finish` renders the
/// `Next:` line and a failure's recovery against it. `shelf commands list`
/// and `shelf aliases list` read the same command, so they list what the
/// caller can run.
#[must_use]
pub fn run() -> ExitCode {
    let command = Cli::command();
    let file = crumbkit_aliases::file(command.get_name());
    let args = crumbkit_aliases::expand(&command, std::env::args_os(), file.as_deref());
    let (mut cli, command) =
        match crumbkit::parse_from::<Cli, _, _>(command, &GUIDE, SPELLING, args) {
            Ok(parsed) => parsed,
            Err(code) => return code,
        };
    cli.output.set_command(command);
    match cli.resource {
        Resource::Books { command } => match command {
            BooksCommand::List(args) => {
                crumbkit::finish(&cli.output, SPELLING, books::BOOKS_LIST, books::list(&args))
            }
            BooksCommand::Show(args) => {
                crumbkit::finish(&cli.output, SPELLING, books::BOOKS_SHOW, books::show(&args))
            }
        },
        Resource::Commands { command } => match command {
            CommandsCommand::List(args) => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit::commands::COMMANDS_LIST,
                crumbkit::commands::list(&cli.output, &GUIDE, &args),
            ),
        },
        Resource::Schemas { command } => match command {
            SchemasCommand::List(args) => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit::schemas::SCHEMAS_LIST,
                crumbkit::schemas::list(&cli.output, &GUIDE, &args),
            ),
            SchemasCommand::Show(args) => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit::schemas::SCHEMAS_SHOW,
                crumbkit::schemas::show(&cli.output, &GUIDE, &args),
            ),
        },
        Resource::SelfCommand { command } => match command {
            SelfCommand::Usage(args) => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit::usage::SELF_USAGE,
                Ok::<_, crumbkit::Error>(crumbkit::usage::findings(&cli.output, &args)),
            ),
            SelfCommand::Figures {
                command: FiguresCommand::List(args),
            } => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit::usage::SELF_FIGURES_LIST,
                Ok::<_, crumbkit::Error>(crumbkit::usage::figures(&cli.output, &args)),
            ),
        },
        Resource::Aliases { command } => match command {
            AliasesCommand::List(args) => crumbkit::finish(
                &cli.output,
                SPELLING,
                crumbkit_aliases::ALIASES_LIST,
                crumbkit_aliases::list(&cli.output, file.as_deref(), &args),
            ),
        },
    }
}
