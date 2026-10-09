//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.

mod books;
mod cli;

use std::process::ExitCode;

use clap::CommandFactory as _;
use crumbkit::Spelling;

use crate::cli::{BooksCommand, Cli, GUIDE, Resource};

#[cfg(test)]
mod tests;

/// How shelf spells its failure codes, such as `shelf::books::not_found`.
/// shelf's test passes the same spelling to crumbkit-test's codes check.
const SPELLING: Spelling = Spelling::COLONS;

/// Parses the command line and runs the command. `crumbkit::help::extend`
/// gives the command tree shelf's goal headings and help topics before
/// parsing.
/// `crumbkit::parse` prints help, the version or a refused command line and
/// returns the exit code. `crumbkit::finish` prints the result or the failure
/// in the format that the output flags choose and returns the process's exit
/// code.
///
/// shelf passes `set_command` the command that `crumbkit::parse` returns,
/// which holds crumbkit's help lines, and `finish` renders the `Next:` line
/// and a failure's recovery against it.
fn main() -> ExitCode {
    let command = crumbkit::help::extend(Cli::command(), &GUIDE);
    let (mut cli, command) = match crumbkit::parse::<Cli>(command, SPELLING) {
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
    }
}
