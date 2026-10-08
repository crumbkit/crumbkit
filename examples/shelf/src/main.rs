//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.

mod books;
mod cli;

use std::process::ExitCode;

use clap::Parser as _;

use crate::cli::{BooksCommand, Cli, Resource};

/// Parses the command line and runs the command. `crumbkit::finish` prints
/// the result in the format that the output flags choose and returns the
/// process's exit code.
fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.resource {
        Resource::Books {
            command: BooksCommand::List,
        } => crumbkit::finish(&cli.output, books::BOOKS_LIST, books::list()),
    }
}
