//! The branch export: `<tool> commands list [PATH]...`, which lists one
//! branch of a tool's command tree with each command's arguments, examples
//! and columns, so an agent loads a subtree in one call instead of reading
//! its help page by page (ADR-0026).
//!
//! The tool owns the command. It adds a `commands` resource with a `list`
//! subcommand that takes [`CommandsList`] to its own clap types, and its
//! handler passes the result of [`list`] to [`finish`](crate::finish) with
//! the document [`COMMANDS`]. A tool that does not want the command leaves
//! it out.
//!
//! ```
//! use std::process::ExitCode;
//!
//! use clap::{CommandFactory, Parser, Subcommand};
//! use crumbkit::commands::{self, CommandsList};
//! use crumbkit::help::Guide;
//! use crumbkit::{Output, Spelling};
//!
//! /// Browse a small catalogue.
//! #[derive(Parser)]
//! #[command(name = "shelf")]
//! struct Cli {
//!     #[command(subcommand)]
//!     resource: Resource,
//!     #[command(flatten)]
//!     output: Output,
//! }
//!
//! #[derive(Subcommand)]
//! enum Resource {
//!     /// Explore shelf's commands.
//!     Commands {
//!         #[command(subcommand)]
//!         command: CommandsCommand,
//!     },
//! }
//!
//! #[derive(Subcommand)]
//! enum CommandsCommand {
//!     /// List one branch of shelf's commands with their arguments.
//!     List(CommandsList),
//! }
//!
//! const GUIDE: Guide = Guide::new();
//! const SPELLING: Spelling = Spelling::COLONS;
//!
//! fn run(args: &[&str]) -> ExitCode {
//!     let (mut cli, command) =
//!         match crumbkit::parse_from::<Cli, _, _>(Cli::command(), &GUIDE, SPELLING, args) {
//!             Ok(parsed) => parsed,
//!             Err(code) => return code,
//!         };
//!     cli.output.set_command(command);
//!     match cli.resource {
//!         Resource::Commands { command: CommandsCommand::List(args) } => crumbkit::finish(
//!             &cli.output,
//!             SPELLING,
//!             commands::COMMANDS,
//!             commands::list(&cli.output, &GUIDE, &args),
//!         ),
//!     }
//! }
//!
//! assert_eq!(run(&["shelf", "commands", "list"]), ExitCode::SUCCESS);
//! ```

mod row;

use std::borrow::Cow;
use std::fmt;

use clap::{Args, Command};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;

pub use self::row::CommandRow;
use crate::help::Guide;
use crate::{Call, Code, Document, Error, Failure, Listing, Output, Page, Recovery};

#[cfg(test)]
mod tests;

/// The document that `commands list` prints: one page of the rows of a
/// branch and the number of visible commands in it. Its schema is
/// `crates/crumbkit/schemas/commands-v1.json`.
pub const COMMANDS: Document<Listing<CommandRow>> = Document::new("crumbkit", "commands", 1);

/// The `crumbkit/commands/v1` document as `finish` writes a listing of
/// command rows with `--json`: the id, the rows and the number of commands
/// in the branch. The type exists only for its schema.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. `crates/crumbkit/schemas/commands-v1.json` holds the schema, and
/// a test fails when the two differ (ADR-0007).
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "only the schema test reads the type until a tool serves its schemas (ADR-0007)"
    )
)]
pub(crate) struct CommandsDocument;

impl JsonSchema for CommandsDocument {
    fn schema_name() -> Cow<'static, str> {
        "CommandsDocument".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit/commands/v1".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The document that `commands list` prints with --json: one row for each visible command of a branch of the command tree.",
            "type": "object",
            "properties": {
                "schema": {
                    "description": "The id of this document.",
                    "const": "crumbkit/commands/v1",
                },
                "data": {
                    "description": "The commands of the branch, in depth-first order, up to the limit.",
                    "type": "array",
                    "items": CommandRow::json_schema(generator),
                },
                "total": {
                    "description": "The number of visible commands in the branch.",
                    "type": "integer",
                    "minimum": 0,
                },
            },
            "required": ["schema", "data", "total"],
        })
    }
}

/// The code of a `PATH` that names no visible command, in the tool's
/// spelling, such as `crumbkit::commands::not_found` (ADR-0026).
const NOT_FOUND: Code = Code::new("crumbkit", "commands", "not_found");

/// The code of a `commands list` run in a tool that never passed its
/// command to `Output::set_command`, in the tool's spelling, such as
/// `crumbkit::commands::unavailable` (ADR-0026).
const UNAVAILABLE: Code = Code::new("crumbkit", "commands", "unavailable");

/// The path from the root to `commands list`, which the recovery of a
/// `PATH` that names no visible command runs.
const COMMANDS_LIST: &[&str] = &["commands", "list"];

/// The arguments of `<tool> commands list`: the names of the commands that
/// lead from the root to the branch, and the paging flags.
///
/// A tool's `commands list` subcommand takes it as its arguments type, as
/// in the [module example](self).
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct CommandsList {
    /// The names of the commands below the root that lead to the branch, such as `books list`
    #[arg(value_name = "PATH")]
    path: Vec<String>,

    /// How many commands to print.
    #[command(flatten)]
    page: Page,
}

/// Serializes the arguments as an object with `path` beside the page's
/// `limit` and `all`, such as `{"path":["books"],"limit":25,"all":false}`,
/// as `#[serde(flatten)]` would.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for CommandsList {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("path", &self.path)?;
        self.page.serialize_fields(&mut map)?;
        map.end()
    }
}

/// Describes the object that `Serialize` writes: the properties of `Page`'s
/// schema with `path` added.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature.
impl JsonSchema for CommandsList {
    fn schema_name() -> Cow<'static, str> {
        "CommandsList".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::commands::CommandsList".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let mut schema = <Page as JsonSchema>::json_schema(generator);
        let path = json_schema!({
            "description": "The names of the commands below the root that lead to the branch, such as `books list`.",
            "type": "array",
            "items": { "type": "string" },
        });
        if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
            properties.insert("path".to_owned(), path.to_value());
        }
        if let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut) {
            required.insert(0, Value::from("path"));
        }
        schema
    }
}

/// Returns the rows of the branch that `args` names in the command tree
/// that the tool passed to [`Output::set_command`], one page of them as
/// `args` asks, for the tool to pass to [`finish`](crate::finish) with
/// [`COMMANDS`].
///
/// The branch is the command that the words of `args` name below the root,
/// or the root when there are none, and then every visible command below it
/// in depth-first order. A hidden command and everything below it are left
/// out, as help leaves them out. Each row holds the command's path from the
/// tool's name, its about texts, its arguments, and the examples and
/// columns of the leaf in `guide` whose path names it.
///
/// A global argument is listed once: on the command that defines it, or on
/// the branch's first row when a command above the branch defines it, so a
/// branch such as `shelf commands list books` shows `--json`.
///
/// The tree that [`parse`](crate::parse) returns has no `help`
/// subcommands. A tool that builds its tree with plain clap and keeps
/// clap's generated `help` subcommands sees them listed as commands.
///
/// The work grows with the size of the branch, and happens only when the
/// caller runs `commands list`. It builds rows for the page only, and
/// builds a row type's JSON Schema once for each leaf with a document on
/// the page.
///
/// # Errors
///
/// Returns a failure with the code `crumbkit::commands::not_found` when a
/// word of `args` names no visible command. Its recovery runs
/// `commands list` with the longest start of the words that names a
/// visible command, such as `shelf commands list books`, rendered against
/// the tool's command, so it has no next step when the tool hides or lacks
/// `commands list`.
///
/// Returns a failure with the code `crumbkit::commands::unavailable` and no
/// next step when the tool has not passed its command to
/// [`Output::set_command`].
pub fn list(
    output: &Output,
    guide: &Guide,
    args: &CommandsList,
) -> Result<Listing<CommandRow>, Error> {
    let Some(root) = output.command() else {
        return Err(CommandsError::Unavailable.into());
    };
    let mut node = root;
    let mut path = Vec::new();
    for word in &args.path {
        match node.find_subcommand(word) {
            Some(subcommand) if !subcommand.is_hide_set() => {
                node = subcommand;
                path.push(subcommand.get_name());
            }
            _ => {
                let line = std::iter::once(root.get_name())
                    .chain(path.iter().copied())
                    .chain(std::iter::once(word.as_str()))
                    .collect::<Vec<_>>()
                    .join(" ");
                let prefix = path.iter().map(|&name| name.to_owned()).collect();
                return Err(CommandsError::NotFound { line, prefix }.into());
            }
        }
    }
    // The branch's first row has no parent, so it lists the global
    // arguments that it inherits from the commands above the branch.
    let mut visits = Vec::new();
    visit(node, None, path, &mut visits);
    let total = visits.len();
    let shown = args.page.limit().unwrap_or(total);
    let rows = visits
        .iter()
        .take(shown)
        .map(|visit| {
            CommandRow::new(
                root.get_name(),
                &visit.path,
                visit.command,
                visit.parent,
                guide,
            )
        })
        .collect();
    Ok(Listing::new(rows, total).described("commands"))
}

/// One visible command of the branch, with what its row needs to know.
struct Visit<'a> {
    /// The command.
    command: &'a Command,
    /// The command above it, or `None` for the branch's first command.
    parent: Option<&'a Command>,
    /// The real names of the subcommands from the root to the command.
    path: Vec<&'a str>,
}

/// Adds `command`, at `path` below `parent`, to `visits`, and then each
/// visible command below it in depth-first order, leaving out a hidden
/// command and everything below it.
fn visit<'a>(
    command: &'a Command,
    parent: Option<&'a Command>,
    path: Vec<&'a str>,
    visits: &mut Vec<Visit<'a>>,
) {
    let below = path.clone();
    visits.push(Visit {
        command,
        parent,
        path,
    });
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }
        let mut path = below.clone();
        path.push(subcommand.get_name());
        visit(subcommand, Some(command), path, visits);
    }
}

/// A failure of `commands list`.
#[derive(Debug)]
enum CommandsError {
    /// A word of `PATH` names no visible command.
    NotFound {
        /// The command line up to and including the word, such as
        /// `shelf books shw`.
        line: String,
        /// The real names of the visible commands that the words before it
        /// name.
        prefix: Vec<String>,
    },
    /// The tool has not passed its command to `Output::set_command`.
    Unavailable,
}

/// Prints the message, such as ``` `shelf books shw` names no visible
/// command ```. A hidden command prints as a missing one, so the message
/// never names a command that the tool hid.
impl fmt::Display for CommandsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { line, .. } => write!(f, "`{line}` names no visible command"),
            Self::Unavailable => f.write_str(
                "the tool has not kept its command tree, so it cannot list its commands",
            ),
        }
    }
}

impl std::error::Error for CommandsError {}

impl Failure for CommandsError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &NOT_FOUND,
            Self::Unavailable => &UNAVAILABLE,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NotFound { prefix, .. } => {
                Recovery::Run(prefix.iter().fold(Call::new(COMMANDS_LIST), |call, name| {
                    call.word(name.clone())
                }))
            }
            Self::Unavailable => Recovery::None {
                reason: "the tool's main does not pass its command to Output::set_command"
                    .to_owned(),
            },
        }
    }
}
