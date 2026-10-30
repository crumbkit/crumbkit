//! reading is crumbkit's example of a tool that adds commands at run time.
//! It keeps reading lists as text files in a directory, and adds a command
//! for each list, such as `reading to-read list`.
//!
//! The library holds the whole tool, and the `reading` binary calls [`run`].
//! It exports the functions that build the command tree and the guide from a
//! directory's lists, so reading's tests can pass the tree and the guide of
//! their fixture lists to crumbkit-test's checks.

mod cli;
mod graft;
mod lists;

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::SystemTime;

pub use crate::cli::{Cli, GUIDE};
use crate::cli::{CommandsCommand, EntriesCommand, ListsCommand, Resource, SchemasCommand};
use crate::graft::{ListCommand, Parsed};
pub use crate::graft::{command, guide};
pub use crate::lists::{List, lists_in};
pub use crate::lists::{NOT_FOUND, OUT_OF_RANGE, UNREADABLE};

#[cfg(test)]
mod tests;

/// Every failure code that reading declares, which reading's tests pass to
/// crumbkit-test's codes and recovery checks.
pub const CODES: &[&crumbkit::Code] = lists::EntriesError::CODES;

/// Parses the process's command line, runs the command and returns the
/// process's exit code.
#[must_use]
pub fn run() -> ExitCode {
    run_from(std::env::args_os())
}

/// Runs reading on `args` as [`run`] runs it on the process's command line,
/// and returns the process's exit code. The first argument is the program
/// name, as in `crumbkit::parse_from`.
///
/// Before parsing, reading reads the first line of each list in its
/// directory and adds a command for each list to its tree, and a leaf with
/// the command's examples to its guide. It passes the same guide to
/// `commands list` and `schemas`, so they describe the commands it added.
#[must_use]
pub fn run_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let dir = lists::dir();
    let lists = dir.as_deref().map(lists_in).unwrap_or_default();
    let guide = guide(&lists);
    let (parsed, mut output) =
        match crumbkit::parse_from::<Parsed, _, _>(command(&lists), &guide, args) {
            Ok(parsed) => parsed,
            Err(stop) => return stop.code(),
        };
    match parsed {
        Parsed::Grafted {
            name,
            command: ListCommand::List(args),
        } => {
            let result = lists::entries(dir.as_deref(), &name, &args.page);
            crumbkit::finish(output, lists::ENTRIES_LIST, result)
        }
        Parsed::Grafted {
            name,
            command: ListCommand::Edit,
        } => {
            let editor = lists::editor(dir.as_deref(), &name);
            crumbkit_child::hand_over(output, editor)
        }
        Parsed::Static(cli) => {
            match cli.resource {
                Resource::Entries {
                    command: EntriesCommand::List(args),
                } => {
                    let result = lists::entries(dir.as_deref(), &args.list, &args.page);
                    crumbkit::finish(output, lists::ENTRIES_LIST, result)
                }
                Resource::Entries {
                    command: EntriesCommand::Stream(args),
                } => lists::stream_entries(output, dir.as_deref(), &args.list),
                Resource::Entries {
                    command: EntriesCommand::Export(args),
                } => {
                    let result = lists::export(dir.as_deref(), &args.list);
                    crumbkit::finish(output, lists::ENTRIES_EXPORT, result)
                }
                // `set` reads `--dry-run` and asks for confirmation through
                // `output`, so it runs before `finish` takes it.
                Resource::Entries {
                    command: EntriesCommand::Set(args),
                } => {
                    let result = lists::set(&mut output, dir.as_deref(), &args);
                    crumbkit::finish(output, lists::ENTRIES_SET, result)
                }
                Resource::Lists {
                    command: ListsCommand::List(args),
                } => {
                    // The age of each list counts to the time the command
                    // runs.
                    let now = SystemTime::now();
                    let summaries = lists::summaries(dir.as_deref(), &args.page, now);
                    let result = Ok::<_, lists::EntriesError>(summaries);
                    crumbkit::finish(output, lists::LISTS_LIST, result)
                }
                Resource::Lists {
                    command: ListsCommand::Show(args),
                } => {
                    let result =
                        Ok::<_, lists::EntriesError>(lists::show(dir.as_deref(), &args.names));
                    crumbkit::finish(output, lists::LISTS_SHOW, result)
                }
                // `commands list` and `schemas` read the command tree
                // through `&mut output`, so each runs before `finish`
                // takes `output` in turn.
                Resource::Commands {
                    command: CommandsCommand::List(args),
                } => {
                    let result = crumbkit::commands::list(&mut output, &guide, &args);
                    let document = crumbkit::commands::COMMANDS_LIST;
                    crumbkit::finish(output, document, result)
                }
                Resource::Where => {
                    let location = lists::Location::of(dir.as_deref());
                    // Finding the directory cannot fail, so the error
                    // type is crumbkit's own and never occurs.
                    let result = Ok::<_, crumbkit::Error>(crumbkit::Prose::new(location));
                    crumbkit::finish(output, lists::WHERE, result)
                }
                Resource::Schemas { command } => match command {
                    SchemasCommand::List(args) => {
                        let result = crumbkit::schemas::list(&mut output, &guide, &args);
                        let document = crumbkit::schemas::SCHEMAS_LIST;
                        crumbkit::finish(output, document, result)
                    }
                    SchemasCommand::Show(args) => {
                        let result = crumbkit::schemas::show(&mut output, &guide, &args);
                        let document = crumbkit::schemas::SCHEMAS_SHOW;
                        crumbkit::finish(output, document, result)
                    }
                },
            }
        }
    }
}
