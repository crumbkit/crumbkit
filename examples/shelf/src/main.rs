//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.

mod books;
mod cli;

use std::process::ExitCode;

use clap::{CommandFactory as _, Parser as _};
use crumbkit::Spelling;

use crate::cli::{BooksCommand, Cli, Resource};

#[cfg(test)]
mod tests;

/// How shelf spells its failure codes, such as `shelf::books::not_found`.
/// shelf's test passes the same spelling to crumbkit-test's codes check.
const SPELLING: Spelling = Spelling::COLONS;

/// Parses the command line and runs the command. `crumbkit::finish` prints
/// the result or the failure in the format that the output flags choose and
/// returns the process's exit code.
///
/// shelf does not change its command tree at runtime, so it gives
/// `set_command` the same `Cli::command()` that `Cli::parse` parsed against,
/// and `finish` renders the `Next:` line and a failure's recovery against it.
fn main() -> ExitCode {
    let mut cli = Cli::parse();
    cli.output.set_command(Cli::command());
    match cli.resource {
        Resource::Books { command } => match command {
            BooksCommand::List(args) => {
                crumbkit::finish(&cli.output, SPELLING, books::BOOKS_LIST, books::list(&args))
            }
            BooksCommand::Show(args) => {
                crumbkit::finish(&cli.output, SPELLING, books::BOOKS_SHOW, books::show(&args))
            }
        },
    }
}
