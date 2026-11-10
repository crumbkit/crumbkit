//! shelf is crumbkit's example tool. It browses a small built-in catalogue of
//! public-domain books, and needs no network or credentials.
//!
//! The library holds the whole tool, and the `shelf` binary calls [`run`].
//! It exports the parser and the tool record, so shelf's integration tests can
//! pass them to crumbkit-test's checks beside the built binary.

mod books;
mod cli;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::CommandFactory as _;

pub use crate::books::{
    BAD_IDS_FILE, EMPTY_IDS_FILE, NO_STATE_DIRECTORY, NOT_FOUND, REMOVED_FILE, TOO_MANY_IDS,
    UNREADABLE_IDS_FILE,
};
use crate::cli::{
    AliasesCommand, BooksCommand, CommandsCommand, FiguresCommand, Resource, RunsCommand, SKILL,
    SchemasCommand, SelfCommand, SkillsCommand,
};
pub use crate::cli::{Cli, TOOL};

#[cfg(test)]
mod tests;

/// Every failure code that shelf declares, which shelf's tests pass to
/// crumbkit-test's codes and recovery checks.
pub const CODES: &[&crumbkit::Code] = books::BooksError::CODES;

/// Parses the process's command line, runs the command and returns the
/// process's exit code. `crumbkit::try_parse_to` takes shelf's tool record, which
/// gives its help goal headings and help topics. It prints help, the version or
/// a refused command line and returns the exit code. `crumbkit::finish` prints
/// the result or the failure in the format that the output flags choose and
/// returns the process's exit code.
///
/// Before parsing, `crumbkit_aliases::expand` replaces a personal alias in
/// the first word with its words from shelf's aliases file, so clap,
/// help and every output see only real commands.
///
/// `crumbkit::try_parse_to` returns the parsed `Cli` and a `Run`. The
/// run holds the command with crumbkit's help lines and the words that
/// the caller typed, which `books remove` repeats with `--yes` when no
/// person can confirm it. `finish` renders the
/// `Next:` line and a failure's recovery against it. `shelf commands list`
/// and `shelf aliases list` read the same command, so they list what the
/// caller can run.
#[must_use]
pub fn run() -> ExitCode {
    run_from(std::env::args_os())
}

/// Runs shelf on `args` as [`run`] runs it on the process's command line,
/// and returns the process's exit code. The first argument is the program
/// name, as in `crumbkit::parse_from`. The output goes to the process's
/// standard output and standard error.
///
/// crumbkit's benchmarks call it, so they measure the path that every run
/// of the `shelf` binary takes (ADR-0008).
#[must_use]
pub fn run_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let command = Cli::command();
    let file = crumbkit_aliases::file(command.get_name());
    let args = crumbkit_aliases::expand(&command, args, file.as_deref());
    // The aliases needed the tree, so the parse takes that tree and the run
    // builds it once.
    let parsed =
        crumbkit::try_parse_to::<Cli>(&mut crumbkit::Sink::standard(), command, &TOOL, args);
    let (cli, mut run) = match parsed.map_err(crumbkit::Unparsed::print) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    match cli.resource {
        Resource::Books { command } => {
            let removed = books::removed_file();
            let removed = removed.as_deref();
            match command {
                BooksCommand::List(args) => {
                    crumbkit::finish(run, books::BOOKS_LIST, books::list(&args, removed))
                }
                BooksCommand::Show(args) => {
                    let room = run.room(books::BOOKS_SHOW);
                    let result = books::show(&args, removed, room);
                    crumbkit::finish(run, books::BOOKS_SHOW, result)
                }
                BooksCommand::Summarize(args) => crumbkit::finish(
                    run,
                    books::BOOKS_SUMMARIZE,
                    books::summarize(&args, removed),
                ),
                // `remove` reads `--dry-run` and asks for confirmation
                // through `&mut run`, so it runs before `finish`.
                BooksCommand::Remove(args) => {
                    let result = books::remove(&mut run, &args, removed);
                    crumbkit::finish(run, books::BOOKS_REMOVE, result)
                }
            }
        }
        // `commands list`, `schemas`, `self` and `aliases list` read the
        // command tree through `&mut run`, so each runs before
        // `finish` takes `run` in turn.
        Resource::Commands { command } => match command {
            CommandsCommand::List(args) => {
                let result = crumbkit::commands::list(&mut run, &TOOL, &args);
                let document = crumbkit::commands::COMMANDS_LIST;
                crumbkit::finish(run, document, result)
            }
        },
        Resource::Schemas { command } => match command {
            SchemasCommand::List(args) => {
                let result = crumbkit::schemas::list(&mut run, &TOOL, &args);
                let document = crumbkit::schemas::SCHEMAS_LIST;
                crumbkit::finish(run, document, result)
            }
            SchemasCommand::Show(args) => {
                let result = crumbkit::schemas::show(&mut run, &TOOL, &args);
                let document = crumbkit::schemas::SCHEMAS_SHOW;
                crumbkit::finish(run, document, result)
            }
        },
        Resource::SelfCommand { command } => match command {
            SelfCommand::Usage(args) => {
                let result =
                    Ok::<_, crumbkit::Error>(crumbkit_usage::findings(&mut run, &TOOL, &args));
                let document = crumbkit_usage::SELF_USAGE;
                crumbkit::finish(run, document, result)
            }
            SelfCommand::Figures {
                command: FiguresCommand::List(args),
            } => {
                let result =
                    Ok::<_, crumbkit::Error>(crumbkit_usage::figures(&mut run, &TOOL, &args));
                let document = crumbkit_usage::SELF_FIGURES_LIST;
                crumbkit::finish(run, document, result)
            }
            SelfCommand::Runs {
                command: RunsCommand::List(args),
            } => {
                let result = Ok::<_, crumbkit::Error>(crumbkit_usage::runs(&mut run, &TOOL, &args));
                let document = crumbkit_usage::SELF_RUNS_LIST;
                crumbkit::finish(run, document, result)
            }
        },
        Resource::Skills { command } => match command {
            SkillsCommand::Install(args) => {
                let result = Ok::<_, crumbkit::Error>(crumbkit_skills::install(&SKILL, &args));
                crumbkit::finish(run, crumbkit_skills::SKILLS_INSTALL, result)
            }
            SkillsCommand::Check(args) => {
                let result = Ok::<_, crumbkit::Error>(crumbkit_skills::check(&SKILL, &args));
                crumbkit::finish(run, crumbkit_skills::SKILLS_CHECK, result)
            }
            SkillsCommand::Print => {
                let result = Ok::<_, crumbkit::Error>(crumbkit_skills::print(&SKILL));
                crumbkit::finish(run, crumbkit_skills::SKILLS_PRINT, result)
            }
        },
        Resource::Aliases { command } => match command {
            AliasesCommand::List(args) => {
                let result = crumbkit_aliases::list(&mut run, file.as_deref(), &args);
                let document = crumbkit_aliases::ALIASES_LIST;
                crumbkit::finish(run, document, result)
            }
        },
    }
}
