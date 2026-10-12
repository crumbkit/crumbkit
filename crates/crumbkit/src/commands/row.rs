//! The row that `commands list` prints for each command of a branch, and
//! how it is read from the runtime command tree and the tool's guide.

use std::borrow::Cow;

use clap::{Arg, ArgAction, Command};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::Row;
use crate::help::leaf::quote;
use crate::help::{Effects, Guide, Leaf, effects};

#[cfg(test)]
mod tests;

/// One visible command of a branch, as `commands list` prints it: what the
/// command's `-h` and `--help` show, apart from layout.
///
/// The text table shows the path and the about text. The arguments,
/// examples, columns, exit codes and effects are lists or objects, which a
/// table cell prints as compact JSON, so they appear with `--json --full` or
/// `--fields`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRow {
    // The `JsonSchema` impl below describes each field of this struct and of
    // the structs it holds, and it is the one place those descriptions live.
    path: String,
    about: Option<String>,
    long_about: Option<String>,
    arguments: Vec<Argument>,
    examples: Vec<CommandExample>,
    columns: Vec<CommandColumn>,
    form: Option<&'static str>,
    exits: Vec<CommandExit>,
    effects: Option<Effects>,
}

/// One argument of a command, as the command's help shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Argument {
    // `CommandRow`'s `JsonSchema` impl describes each field.
    name: String,
    short: Option<String>,
    value_names: Vec<String>,
    required: bool,
    many: bool,
    defaults: Vec<String>,
    possible_values: Vec<String>,
    help: Option<String>,
    long_help: Option<String>,
    global: bool,
    one_of: Vec<String>,
}

/// One example of a command, from the guide's leaf for the command.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandExample {
    // `CommandRow`'s `JsonSchema` impl describes each field.
    command: String,
    about: &'static str,
}

/// One column of the result that a command prints, from the document of
/// the guide's leaf for the command.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandColumn {
    // `CommandRow`'s `JsonSchema` impl describes each field.
    name: &'static str,
    long: bool,
    description: Option<String>,
}

/// One exit code that the guide declares for the command, besides
/// crumbkit's 0, 1 and 2.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandExit {
    // `CommandRow`'s `JsonSchema` impl describes each field.
    exit: u8,
    meaning: &'static str,
}

impl CommandRow {
    /// Returns the row of `command`, whose path from the root of `tool`'s
    /// tree is `path`, the real names of its subcommands. `parent` is the
    /// command above it, or `None` for the first command of the branch,
    /// whose row then lists every global argument it has, including those
    /// it inherits. `guide` holds the leaf whose examples and columns the
    /// row lists and the exit codes that cover the command.
    ///
    /// The row reads only the command's name, about texts and arguments,
    /// and the guide. It never reads a command's help template,
    /// `before_help` or `after_help`, where `crumbkit::parse` adds its help
    /// lines on the walked path, and it reads `Arg::is_hide_set`, which
    /// `parse` never changes, so a row is the same whichever path the
    /// caller typed.
    pub(super) fn new(
        tool: &str,
        path: &[&str],
        command: &Command,
        parent: Option<&Command>,
        guide: &Guide,
    ) -> Self {
        let line = std::iter::once(tool)
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        let leaf = guide.get_leaves().iter().find(|leaf| leaf.path() == path);
        let examples = leaf.map_or_else(Vec::new, |leaf| {
            leaf.get_examples()
                .iter()
                .map(|example| CommandExample {
                    command: example.words().iter().fold(line.clone(), |mut text, word| {
                        text.push(' ');
                        text.push_str(&quote(word));
                        text
                    }),
                    about: example.about(),
                })
                .collect()
        });
        let columns = leaf.map_or_else(Vec::new, |leaf| {
            leaf.column_descriptions()
                .into_iter()
                .map(|column| CommandColumn {
                    name: column.name(),
                    long: column.is_long(),
                    description: column.description().map(ToOwned::to_owned),
                })
                .collect()
        });
        let exits = guide
            .get_exits()
            .iter()
            .filter(|exit| exit.covers(path))
            .map(|exit| CommandExit {
                exit: exit.code(),
                meaning: exit.meaning(),
            })
            .collect();
        let arguments = command
            .get_arguments()
            .filter(|arg| !arg.is_hide_set() && !prints_help(arg) && !is_inherited(arg, parent))
            .map(|arg| Argument::new(arg, command))
            .collect();
        Self {
            path: line,
            about: command
                .get_about()
                .or_else(|| command.get_long_about())
                .map(ToString::to_string),
            long_about: command.get_long_about().map(ToString::to_string),
            arguments,
            examples,
            columns,
            form: leaf.and_then(form),
            exits,
            effects: leaf.and_then(Leaf::get_effects),
        }
    }
}

/// Returns the form of the document that `leaf` declares, as `commands
/// list` writes it: `stream`, `document`, or `None` when the leaf declares
/// no document.
fn form(leaf: &Leaf) -> Option<&'static str> {
    leaf.get_document()?;
    Some(if leaf.is_stream() {
        "stream"
    } else {
        "document"
    })
}

impl Argument {
    /// Returns `arg` as its command's help shows it, with the other members
    /// of each required group of `command` that holds it. `arg` belongs to
    /// `command`, which is built, because clap sets an argument's number of
    /// values during the build.
    //
    // RUST-10: the argument leaves out the environment variable that help
    // shows beside it, because `Arg::get_env` exists only with clap's `env`
    // feature, which crumbkit does not enable. A crumbkit feature that
    // enables clap's `env` and adds an `env` field replaces it.
    fn new(arg: &Arg, command: &Command) -> Self {
        let takes_values = arg.get_action().takes_values();
        let value_names: Vec<String> = if takes_values {
            arg.get_value_names().map_or_else(
                || vec![arg.get_id().to_string()],
                |names| names.iter().map(ToString::to_string).collect(),
            )
        } else {
            Vec::new()
        };
        let defaults = if takes_values && !arg.is_hide_default_value_set() {
            arg.get_default_values()
                .iter()
                .map(|value| value.to_string_lossy().into_owned())
                .collect()
        } else {
            Vec::new()
        };
        let possible_values = if takes_values && !arg.is_hide_possible_values_set() {
            arg.get_possible_values()
                .iter()
                .filter(|value| !value.is_hide_set())
                .map(|value| value.get_name().to_owned())
                .collect()
        } else {
            Vec::new()
        };
        Self {
            name: name(arg),
            short: arg.get_short().map(|short| format!("-{short}")),
            value_names,
            required: arg.is_required_set(),
            many: arg
                .get_num_args()
                .is_some_and(|range| range.max_values() > 1)
                || matches!(arg.get_action(), ArgAction::Append),
            defaults,
            possible_values,
            help: arg
                .get_help()
                .or_else(|| arg.get_long_help())
                .map(ToString::to_string),
            long_help: arg.get_long_help().map(ToString::to_string),
            global: arg.is_global_set(),
            one_of: one_of(arg, command),
        }
    }
}

/// Returns the name of `arg` as help writes it: `--all` for an argument with
/// a long name, `-a` for one with only a short name, and `<ID>` for a
/// positional argument, from its first value name or else its id.
fn name(arg: &Arg) -> String {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => {
            let value_name = arg
                .get_value_names()
                .and_then(|names| names.first())
                .map_or_else(|| arg.get_id().to_string(), ToString::to_string);
            format!("<{value_name}>")
        }
    }
}

/// Returns the names of the visible arguments that can be given in place of
/// `arg`. These are the other members of each required group of `command`
/// that holds `arg`, in the order the groups list them, and then, when `arg`
/// is required, the arguments it conflicts with. The command refuses a
/// command line that gives none of a required group's arguments. clap does
/// not ask for a required argument when an argument it conflicts with is
/// given. So `arg` is needed only when none of these names is given.
//
// RUST-10: a group may name another group as a member, and this reads only
// the members that are arguments of `command`, so it leaves a nested group
// out. Reading the members of a nested group replaces it once a tool nests
// its groups. This also reads only the conflicts that `arg` declares, and
// leaves out an argument that declares a conflict with `arg`, because clap
// has no getter for the arguments that conflict with a given one. A tool
// that needs those declares the conflict on the required argument.
fn one_of(arg: &Arg, command: &Command) -> Vec<String> {
    let mut names = Vec::new();
    for group in command.get_groups().filter(|group| group.is_required_set()) {
        if !group.get_args().any(|member| member == arg.get_id()) {
            continue;
        }
        for member in group.get_args().filter(|member| *member != arg.get_id()) {
            let Some(other) = command
                .get_arguments()
                .find(|other| other.get_id() == member && !other.is_hide_set())
            else {
                continue;
            };
            let other = name(other);
            if !names.contains(&other) {
                names.push(other);
            }
        }
    }
    if arg.is_required_set() {
        // `get_arg_conflicts_with` panics on a conflict with an id that
        // `command` does not have. clap's debug build asserts that every
        // conflict names an argument or group of the command when the
        // command is built, so a tool whose tests build its command cannot
        // reach that panic.
        for other in command.get_arg_conflicts_with(arg) {
            if other.is_hide_set() {
                continue;
            }
            let other = name(other);
            if !names.contains(&other) {
                names.push(other);
            }
        }
    }
    names
}

/// Reports whether `arg` prints help or the version, as clap's generated
/// `-h`, `--help` and `-V`, `--version` do. Every command has them, so the
/// rows leave them out.
fn prints_help(arg: &Arg) -> bool {
    matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// Reports whether `arg` is a global argument that the build copied from
/// `parent`, so the rows list it once, on the command that defines it.
//
// RUST-10: clap copies a global argument to each subcommand that has no
// argument with the same id, and has no getter that tells the copy from an
// argument the subcommand defines. The check treats a global argument as a
// copy when `parent` has a global argument with the same id. It misses a
// subcommand that defines its own global argument with the id of one above
// it, which it leaves out. A clap getter for copied arguments replaces it.
fn is_inherited(arg: &Arg, parent: Option<&Command>) -> bool {
    arg.is_global_set()
        && parent.is_some_and(|parent| {
            parent
                .get_arguments()
                .any(|above| above.is_global_set() && above.get_id() == arg.get_id())
        })
}

impl Row for CommandRow {
    const COLUMNS: &'static [&'static str] = &["path", "about"];
}

/// Serializes the row as an object with every field, and `null` for a
/// missing text.
///
/// The `Serialize` impls of this module are written by hand so crumbkit
/// needs no `serde_derive`.
impl Serialize for CommandRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("CommandRow", 9)?;
        row.serialize_field("path", &self.path)?;
        row.serialize_field("about", &self.about)?;
        row.serialize_field("long_about", &self.long_about)?;
        row.serialize_field("arguments", &self.arguments)?;
        row.serialize_field("examples", &self.examples)?;
        row.serialize_field("columns", &self.columns)?;
        row.serialize_field("form", &self.form)?;
        row.serialize_field("exits", &self.exits)?;
        row.serialize_field("effects", &self.effects)?;
        row.end()
    }
}

impl Serialize for Argument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut argument = serializer.serialize_struct("Argument", 11)?;
        argument.serialize_field("name", &self.name)?;
        argument.serialize_field("short", &self.short)?;
        argument.serialize_field("value_names", &self.value_names)?;
        argument.serialize_field("required", &self.required)?;
        argument.serialize_field("many", &self.many)?;
        argument.serialize_field("defaults", &self.defaults)?;
        argument.serialize_field("possible_values", &self.possible_values)?;
        argument.serialize_field("help", &self.help)?;
        argument.serialize_field("long_help", &self.long_help)?;
        argument.serialize_field("global", &self.global)?;
        argument.serialize_field("one_of", &self.one_of)?;
        argument.end()
    }
}

impl Serialize for CommandExample {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut example = serializer.serialize_struct("CommandExample", 2)?;
        example.serialize_field("command", &self.command)?;
        example.serialize_field("about", &self.about)?;
        example.end()
    }
}

impl Serialize for CommandExit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut exit = serializer.serialize_struct("CommandExit", 2)?;
        exit.serialize_field("exit", &self.exit)?;
        exit.serialize_field("meaning", &self.meaning)?;
        exit.end()
    }
}

impl Serialize for CommandColumn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut column = serializer.serialize_struct("CommandColumn", 3)?;
        column.serialize_field("name", &self.name)?;
        column.serialize_field("long", &self.long)?;
        column.serialize_field("description", &self.description)?;
        column.end()
    }
}

/// Describes the object that `Serialize` writes. The descriptions are the
/// only ones the fields have, and a leaf's `--help` describes the columns
/// `path` and `about` from them.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. The nested objects are written inline, so the schema has no
/// `$defs`.
impl JsonSchema for CommandRow {
    fn schema_name() -> Cow<'static, str> {
        "CommandRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::commands::CommandRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "One visible command of a branch of the command tree.",
            "type": "object",
            "properties": {
                "path": {
                    "description": "The command line that runs the command, such as `shelf books list`.",
                    "type": "string",
                },
                "about": {
                    "description": "The command's one-line description, or its full description when it has no one-line description.",
                    "type": ["string", "null"],
                },
                "long_about": {
                    "description": "The full description that the command's `--help` shows, when the command has one.",
                    "type": ["string", "null"],
                },
                "arguments": {
                    "description": "The arguments that the command's help shows, in definition order.",
                    "type": "array",
                    "items": argument_schema(),
                },
                "examples": {
                    "description": "The examples of the guide's leaf for the command, in guide order.",
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "description": "The full command line, such as `shelf books list --all`, with each word that a POSIX shell would split or expand between single quotes.",
                                "type": "string",
                            },
                            "about": {
                                "description": "The one-line description, which is empty when the example has none.",
                                "type": "string",
                            },
                        },
                        "required": ["command", "about"],
                    },
                },
                "columns": {
                    "description": "The columns of the result that the command prints, from the document of the guide's leaf for the command.",
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "description": "The column's name, as the row type serializes the field.",
                                "type": "string",
                            },
                            "long": {
                                "description": "Whether `--long` adds the column.",
                                "type": "boolean",
                            },
                            "description": {
                                "description": "The first paragraph of the field's doc comment, when it has one.",
                                "type": ["string", "null"],
                            },
                        },
                        "required": ["name", "long", "description"],
                    },
                },
                "form": {
                    "description": "stream when the command prints a stream, one row on each line as each arrives, document when it prints one document, and null when the guide's leaf for the command declares no document.",
                    "enum": ["document", "stream", null],
                },
                "exits": {
                    "description": "The exit codes that the tool declares for the command besides crumbkit's 0, 1 and 2, in guide order. `<tool> help exit-codes` describes every code.",
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "exit": {
                                "description": "The exit code.",
                                "type": "integer",
                                "minimum": 0,
                                "maximum": 255,
                            },
                            "meaning": {
                                "description": "What the code means, as one sentence.",
                                "type": "string",
                            },
                        },
                        "required": ["exit", "meaning"],
                    },
                },
                "effects": effects::schema(),
            },
            "required": ["path", "about", "long_about", "arguments", "examples", "columns", "form", "exits", "effects"],
        })
    }
}

/// Returns the schema of one argument, which `CommandRow`'s schema lists
/// under `arguments`.
fn argument_schema() -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "name": {
                "description": "The name as help writes it: `--all` for an argument with a long name, `-a` for one with only a short name, and `<ID>` for a positional argument.",
                "type": "string",
            },
            "short": {
                "description": "The short form, such as `-n`, when the argument has one.",
                "type": ["string", "null"],
            },
            "value_names": {
                "description": "The names of the values that the argument takes, such as `N`, and none for a flag.",
                "type": "array",
                "items": { "type": "string" },
            },
            "required": {
                "description": "Whether the command refuses a command line without the argument.",
                "type": "boolean",
            },
            "many": {
                "description": "Whether the argument takes more than one value, through repeats, commas or several words.",
                "type": "boolean",
            },
            "defaults": {
                "description": "The values that the argument takes when the caller leaves it out, as help shows them.",
                "type": "array",
                "items": { "type": "string" },
            },
            "possible_values": {
                "description": "The values that the argument accepts, as help shows them, and none when it accepts any value.",
                "type": "array",
                "items": { "type": "string" },
            },
            "help": {
                "description": "The one-line help: the argument's `help`, or its `long_help` when it has no `help`.",
                "type": ["string", "null"],
            },
            "long_help": {
                "description": "The full help that `--help` shows, when the argument has one.",
                "type": ["string", "null"],
            },
            "global": {
                "description": "Whether the argument also works after every subcommand below the command that defines it.",
                "type": "boolean",
            },
            "one_of": {
                "description": "The visible arguments that can be given in place of this one. These are the other arguments of each required group that holds it and, when the argument is required, the arguments it conflicts with. The list is empty when no argument can be given in its place.",
                "type": "array",
                "items": { "type": "string" },
            },
        },
        "required": [
            "name",
            "short",
            "value_names",
            "required",
            "many",
            "defaults",
            "possible_values",
            "help",
            "long_help",
            "global",
            "one_of",
        ],
    })
}
