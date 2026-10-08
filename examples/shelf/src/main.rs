//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.

mod books;
mod cli;

use std::process::ExitCode;

use clap::{CommandFactory as _, Parser as _};

use crate::cli::{BooksCommand, Cli, Resource};

/// Parses the command line and runs the command. `crumbkit::finish` prints
/// the result in the format that the output flags choose and returns the
/// process's exit code.
///
/// shelf does not change its command tree at runtime, so it gives
/// `set_command` the same `Cli::command()` that `Cli::parse` parsed against,
/// and `finish` renders the `Next:` line against it.
fn main() -> ExitCode {
    let mut cli = Cli::parse();
    cli.output.set_command(Cli::command());
    match cli.resource {
        Resource::Books { command } => match command {
            BooksCommand::List(args) => {
                crumbkit::finish(&cli.output, books::BOOKS_LIST, books::list(&args))
            }
            BooksCommand::Show(args) => {
                crumbkit::finish(&cli.output, books::BOOKS_SHOW, books::show(&args))
            }
        },
    }
}
