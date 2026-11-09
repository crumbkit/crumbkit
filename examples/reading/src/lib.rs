//! reading is crumbkit's example of a tool that adds commands at run time.
//! It keeps reading lists as text files in a directory, and adds a command
//! for each list, such as `reading to-read list`.
//!
//! The library holds the whole tool, and the `reading` binary calls [`run`]. It
//! exports the functions that build the command tree and the tool record from a
//! directory's lists, so reading's tests can pass the tree and the tool record
//! of their fixture lists to crumbkit-test's checks.

mod cli;
mod graft;
mod lists;

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::SystemTime;

pub use crate::cli::{Cli, TOOL};
use crumbkit::{Sink, Unparsed};

use crate::cli::{CommandsCommand, EntriesCommand, ListsCommand, Resource, SchemasCommand};
use crate::graft::{ListCommand, Parsed};
pub use crate::graft::{command, tool};
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
/// the command's examples to its tool record. It passes the same tool record to
/// `commands list` and `schemas`, so they describe the commands it added.
#[must_use]
pub fn run_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let dir = lists::dir();
    let lists = dir.as_deref().map(lists_in).unwrap_or_default();
    let tool = tool(&lists);
    // reading builds its tree at run time, so it names that tree here, which
    // `parse_from` would take from `Parsed`.
    let parsed =
        crumbkit::try_parse_to::<Parsed>(&mut Sink::standard(), command(&lists), &tool, args);
    let (parsed, mut run) = match parsed.map_err(Unparsed::print) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    match parsed {
        Parsed::Grafted {
            name,
            command: ListCommand::List(args),
        } => {
            let result = lists::entries(dir.as_deref(), &name, &args.page);
            crumbkit::finish(run, lists::ENTRIES_LIST, result)
        }
        Parsed::Grafted {
            name,
            command: ListCommand::Edit,
        } => {
            let editor = lists::editor(dir.as_deref(), &name);
            crumbkit_child::hand_over(run, editor)
        }
        Parsed::Static(cli) => {
            match cli.resource {
                Resource::Entries {
                    command: EntriesCommand::List(args),
                } => {
                    let result = lists::entries(dir.as_deref(), &args.list, &args.page);
                    crumbkit::finish(run, lists::ENTRIES_LIST, result)
                }
                Resource::Entries {
                    command: EntriesCommand::Stream(args),
                } => lists::stream_entries(run, dir.as_deref(), &args.list),
                Resource::Entries {
                    command: EntriesCommand::Export(args),
                } => {
                    let result = lists::export(dir.as_deref(), &args.list);
                    crumbkit::finish(run, lists::ENTRIES_EXPORT, result)
                }
                // `set` reads `--dry-run` and asks for confirmation through
                // `run`, so it runs before `finish` takes it.
                Resource::Entries {
                    command: EntriesCommand::Set(args),
                } => {
                    let result = lists::set(&mut run, dir.as_deref(), &args);
                    crumbkit::finish(run, lists::ENTRIES_SET, result)
                }
                Resource::Lists {
                    command: ListsCommand::List(args),
                } => {
                    // The age of each list counts to the time the command
                    // runs.
                    let now = SystemTime::now();
                    let summaries = lists::summaries(dir.as_deref(), &args.page, now);
                    let result = Ok::<_, lists::EntriesError>(summaries);
                    crumbkit::finish(run, lists::LISTS_LIST, result)
                }
                Resource::Lists {
                    command: ListsCommand::Show(args),
                } => {
                    let result =
                        Ok::<_, lists::EntriesError>(lists::show(dir.as_deref(), &args.names));
                    crumbkit::finish(run, lists::LISTS_SHOW, result)
                }
                // `commands list` and `schemas` read the command tree
                // through `&mut run`, so each runs before `finish`
                // takes `run` in turn.
                Resource::Commands {
                    command: CommandsCommand::List(args),
                } => {
                    let result = crumbkit::commands::list(&mut run, &tool, &args);
                    let document = crumbkit::commands::COMMANDS_LIST;
                    crumbkit::finish(run, document, result)
                }
                Resource::Where => {
                    let location = lists::Location::of(dir.as_deref());
                    // Finding the directory cannot fail, so the error
                    // type is crumbkit's own and never occurs.
                    let result = Ok::<_, crumbkit::Error>(crumbkit::Prose::new(location));
                    crumbkit::finish(run, lists::WHERE, result)
                }
                Resource::Schemas { command } => match command {
                    SchemasCommand::List(args) => {
                        let result = crumbkit::schemas::list(&mut run, &tool, &args);
                        let document = crumbkit::schemas::SCHEMAS_LIST;
                        crumbkit::finish(run, document, result)
                    }
                    SchemasCommand::Show(args) => {
                        let result = crumbkit::schemas::show(&mut run, &tool, &args);
                        let document = crumbkit::schemas::SCHEMAS_SHOW;
                        crumbkit::finish(run, document, result)
                    }
                },
            }
        }
    }
}
