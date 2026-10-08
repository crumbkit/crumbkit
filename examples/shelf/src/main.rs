//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.

mod books;
mod cli;

use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser as _;

use crate::cli::{BooksCommand, Cli, Resource};

/// Parses the command line, runs the command and turns its outcome into the
/// process's exit code. This is the only function that touches the standard
/// streams.
fn main() -> ExitCode {
    let cli = Cli::parse();

    #[expect(
        clippy::disallowed_methods,
        reason = "main owns standard output and locks it once for the whole run"
    )]
    let mut stdout = io::stdout().lock();

    match run(&cli, &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        // A reader that stops early, as `shelf books list | head -1` does,
        // closes the pipe after it has read every line it wanted.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            #[expect(
                clippy::disallowed_methods,
                reason = "main owns standard error and reports a failed write there"
            )]
            let mut stderr = io::stderr();
            // The process is already failing, and a failed report has nowhere else to go.
            let _ = writeln!(stderr, "error: cannot write to standard output: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the command that `cli` names and writes its result to `out`.
fn run(cli: &Cli, out: &mut impl Write) -> io::Result<()> {
    match cli.resource {
        Resource::Books {
            command: BooksCommand::List,
        } => books::write_list(&books::CATALOGUE, out)?,
    }
    out.flush()
}
