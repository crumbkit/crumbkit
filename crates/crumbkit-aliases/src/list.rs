//! `<tool> aliases list`, which lists the built-in aliases of the command
//! tree and the entries of the aliases file, with the problem of each entry
//! that does not work.

use std::fmt;
use std::path::Path;

use clap::{Args, Command};
use crumbkit::help::quote;
use crumbkit::{Code, Document, Error, Failure, Listing, Output, Page, Recovery, Row};
use schemars::JsonSchema;
use serde::Serialize;
use toml::de::{DeTable, DeValue};

use crate::entry::{wins, words};

#[cfg(test)]
mod tests;

/// The document that `aliases list` prints: one page of alias rows and the
/// number of aliases. Its schema is
/// `crates/crumbkit-aliases/schemas/aliases-v1.json`.
pub const ALIASES: Document<Listing<AliasRow>> = Document::new("crumbkit", "aliases", 1);

/// The code of an aliases file that exists but cannot be read or parsed, in
/// the tool's spelling, such as `crumbkit::aliases::unreadable` (ADR-0027).
const UNREADABLE: Code = Code::new("crumbkit", "aliases", "unreadable");

/// The code of an `aliases list` run in a tool that never passed its
/// command to `Output::set_command`, in the tool's spelling, such as
/// `crumbkit::aliases::unavailable` (ADR-0027).
const UNAVAILABLE: Code = Code::new("crumbkit", "aliases", "unavailable");

/// The problem of an entry whose name a real command takes.
const SHADOWED: &str = "the tool has a command or built-in alias of this name, which wins";

/// The problem of an entry named `help` in a tool without a command of that
/// name. crumbkit answers the word `help` itself, before any alias expands.
const HELP: &str = "the tool answers the word help with its own help, which wins";

/// The problem of an entry whose value is not a list of one or more words.
const BAD_VALUE: &str = "the value must be a list of one or more words";

/// The arguments of `<tool> aliases list`: the paging flags.
///
/// A tool's hidden `aliases list` subcommand takes it as its arguments
/// type, as in the [crate example](crate).
#[derive(Args, Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct AliasesList {
    /// How many aliases to print.
    #[command(flatten)]
    #[serde(flatten)]
    page: Page,
}

/// One built-in alias or aliases file entry, as `aliases list` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct AliasRow {
    /// The words that a person types after the tool's name, such as `books ls` for a built-in alias or `mine` for an entry of the aliases file.
    name: String,
    /// The command that the alias stands for, without the tool's name, such as `books list`. An entry's words are quoted for a POSIX shell, and the command is empty when the entry's value is not a list of words.
    command: String,
    /// Why the entry does not expand, or empty when the alias works.
    problem: String,
    /// Where the alias comes from: the tool's name for a built-in alias, or the path of the aliases file for an entry.
    from: String,
}

impl Row for AliasRow {
    // Every field is a compact column. A person runs the command to see what
    // each alias stands for and why an entry does not work, so the problem
    // shows without `--long`. The source comes last because a file's path is
    // the longest cell, and the columns before it stay aligned.
    const COLUMNS: &'static [&'static str] = &["name", "command", "problem", "from"];
}

/// Returns the built-in aliases of the command tree that the tool passed to
/// [`Output::set_command`] and the entries of the aliases file at `file`,
/// one page of them as `args` asks, for the tool to pass to
/// [`finish`](crumbkit::finish) with [`ALIASES`].
///
/// The built-in aliases come first, in the depth-first order of the visible
/// tree. A built-in alias is a clap alias, hidden or visible, of a visible
/// subcommand, and its name is its full path below the root, such as
/// `books ls` for the alias `ls` of `books list`. A hidden command and
/// everything below it are left out, and so are the aliases of arguments and
/// the short and long flag aliases of flag subcommands. The entries of the
/// file follow in name order, each with the problem that keeps it from
/// expanding, as [`expand`](crate::expand) decides it.
///
/// A missing file, or no file because the environment gives no config
/// directory, lists the built-in aliases only. The listing's note, which
/// the text output prints after the table, names the file and says when it
/// does not exist yet, or says why the tool has no aliases file.
///
/// # Errors
///
/// Returns a failure with the code `crumbkit::aliases::unreadable` when a
/// file is at `file` but cannot be read or parsed. The message names the
/// path and the reason, and the recovery asks the person to fix or remove
/// the file.
///
/// Returns a failure with the code `crumbkit::aliases::unavailable` and no
/// next step when the tool has not passed its command to
/// [`Output::set_command`].
pub fn list(
    output: &Output,
    file: Option<&Path>,
    args: &AliasesList,
) -> Result<Listing<AliasRow>, Error> {
    let Some(command) = output.command() else {
        return Err(AliasesError::Unavailable.into());
    };
    let text = match file {
        Some(path) => crate::file::read(path).map_err(|error| AliasesError::Unreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?,
        None => None,
    };
    Ok(listing(command, file, text.as_deref(), args)?)
}

/// Returns the page of rows that `args` asks for, with the note that names
/// the file: the built-in aliases of `command`, then the entries of `text`,
/// the aliases file at `file`, in name order. `text` is `None` when no file
/// is at `file`.
///
/// # Errors
///
/// Returns [`AliasesError::Unreadable`] when `text` is not a TOML table.
fn listing(
    command: &Command,
    file: Option<&Path>,
    text: Option<&str>,
    args: &AliasesList,
) -> Result<Listing<AliasRow>, AliasesError> {
    let tool = command.get_name();
    let note = match (file, text) {
        (Some(path), Some(_)) => format!("Aliases file: {}", path.display()),
        (Some(path), None) => format!("Aliases file: {}, which does not exist yet", path.display()),
        (None, _) => {
            format!("{tool} has no aliases file, because the home directory cannot be found.")
        }
    };
    let rows = rows(command, file, text)?;
    Ok(Listing::from_all(rows, args.page.limit())
        .described("aliases")
        .note(note))
}

/// Returns every row: the built-in aliases of `command`, then the entries
/// of `text`, the aliases file at `file`, in name order.
///
/// # Errors
///
/// Returns [`AliasesError::Unreadable`] when `text` is not a TOML table.
fn rows(
    command: &Command,
    file: Option<&Path>,
    text: Option<&str>,
) -> Result<Vec<AliasRow>, AliasesError> {
    let tool = command.get_name();
    let mut rows = Vec::new();
    built_in(command, &[], tool, &mut rows);
    let (Some(path), Some(text)) = (file, text) else {
        return Ok(rows);
    };
    let path = path.display().to_string();
    let table = DeTable::parse(text).map_err(|error| AliasesError::Unreadable {
        reason: parse_reason(text, &error),
        path: path.clone(),
    })?;
    let mut entries: Vec<(&str, &DeValue<'_>)> = table
        .get_ref()
        .iter()
        .map(|(name, value)| (name.get_ref().as_ref(), value.get_ref()))
        .collect();
    // A `DeTable` keeps keys in name order unless some crate in the build
    // enables toml's `preserve_order` feature, which keeps file order, so
    // the rows sort here.
    entries.sort_by_key(|&(name, _)| name);
    for (name, value) in entries {
        let words = words(value);
        let problem = if name == "help" && command.find_subcommand(name).is_none() {
            HELP
        } else if wins(command, name) {
            SHADOWED
        } else if words.is_none() {
            BAD_VALUE
        } else {
            ""
        };
        rows.push(AliasRow {
            name: name.to_owned(),
            command: words.map(|words| join(&words)).unwrap_or_default(),
            problem: problem.to_owned(),
            from: path.clone(),
        });
    }
    Ok(rows)
}

/// Adds a row to `rows` for each alias of each visible subcommand of
/// `command`, which sits at `path` below the root, and then for the visible
/// commands below each one, in depth-first order.
fn built_in(command: &Command, path: &[&str], tool: &str, rows: &mut Vec<AliasRow>) {
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }
        for alias in subcommand.get_all_aliases() {
            rows.push(AliasRow {
                name: join(&[path, &[alias]].concat()),
                command: join(&[path, &[subcommand.get_name()]].concat()),
                problem: String::new(),
                from: tool.to_owned(),
            });
        }
        let below = [path, &[subcommand.get_name()]].concat();
        built_in(subcommand, &below, tool, rows);
    }
}

/// Joins `words` with spaces, quoting each word that a POSIX shell would
/// split or expand.
fn join(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| quote(word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the reason that `text` is not a TOML table: the parser's message,
/// after the line number of the place it points at when it points at one.
fn parse_reason(text: &str, error: &toml::de::Error) -> String {
    let line = error
        .span()
        .and_then(|span| text.get(..span.start))
        .map(|before| before.matches('\n').count() + 1);
    match line {
        Some(line) => format!("line {line}: {}", error.message()),
        None => error.message().to_owned(),
    }
}

/// A failure of `aliases list`.
#[derive(Debug)]
enum AliasesError {
    /// A file is at the aliases file's path but cannot be read or parsed.
    Unreadable {
        /// The path of the file.
        path: String,
        /// The I/O error or the parser's message.
        reason: String,
    },
    /// The tool has not passed its command to `Output::set_command`.
    Unavailable,
}

/// Prints the message, such as `cannot read the aliases file
/// /home/ada/.config/shelf/aliases.toml: line 2: invalid array`.
impl fmt::Display for AliasesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, reason } => {
                write!(f, "cannot read the aliases file {path}: {reason}")
            }
            Self::Unavailable => {
                f.write_str("the tool has not kept its command tree, so it cannot list its aliases")
            }
        }
    }
}

impl std::error::Error for AliasesError {}

impl Failure for AliasesError {
    fn code(&self) -> &'static Code {
        match self {
            Self::Unreadable { .. } => &UNREADABLE,
            Self::Unavailable => &UNAVAILABLE,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::Unreadable { path, .. } => Recovery::Hint(format!("fix or remove {path}")),
            Self::Unavailable => Recovery::None {
                reason: "the tool's main does not pass its command to Output::set_command"
                    .to_owned(),
            },
        }
    }
}
