//! The row that `commands list` prints for each command of a branch, and
//! how it is read from the runtime command tree and the tool record.

use clap::builder::ValueHint;
use clap::{Arg, ArgAction, Command};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use crate::expect::takes_expect;
use crate::help::effects::Guards;
use crate::help::words::quote;
use crate::help::{Effects, Leaf, LeafDocument, effects, goal_sections};
use crate::{Children, Row, Shape, Tool};

mod schema;
#[cfg(test)]
mod tests;

/// One visible command of a branch, as `commands list` prints it: what the
/// command's `-h` and `--help` show, apart from layout.
///
/// The text table shows the path and the about text. The arguments,
/// examples, columns, exit codes, effects, writes and goals are lists or
/// objects, which a table cell prints as compact JSON, so they appear with
/// `--json --full` or `--fields`.
///
/// The row of a command whose help page prints goal headings for its
/// [`Group`](crate::help::Group) has a `goals` field with those headings and
/// the subcommands listed under them, in the page's order. The page prints a
/// heading for a goal that names at least one visible subcommand. Every
/// other row leaves the field out.
///
/// The row of a command whose leaf declares
/// [`Leaf::prints_no_result`](crate::help::Leaf::prints_no_result) has a
/// `prints_no_result` field that is `true`. Every other row leaves the field
/// out, so a caller tells such a command from one whose leaf declares no
/// document, although both rows have the form `null` (ADR-0169).
///
/// Every row has a `built_in` field. It is `true` for a command whose leaf
/// declares a document or stream that crumbkit or one of its add-on crates
/// marks as its own, such as `commands list` or `self usage`, and for a
/// group whose visible commands below it number at least one and are all
/// built in. It is `false` for every other row, such as the tool's own
/// command under its `self` group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRow {
    // The `JsonSchema` impl in `schema.rs` describes each field of this struct and of
    // the structs it holds, and it is the one place those descriptions live.
    path: String,
    about: Option<String>,
    long_about: Option<String>,
    arguments: Vec<Argument>,
    examples: Vec<CommandExample>,
    columns: Vec<CommandColumn>,
    form: Option<&'static str>,
    line: Option<&'static str>,
    children: Option<&'static str>,
    children_help: Option<String>,
    exits: Vec<CommandExit>,
    effects: Option<Effects>,
    writes: Option<Guards>,
    goals: Vec<CommandGoal>,
    prints_no_result: bool,
    built_in: bool,
}

/// One argument of a command, as the command's help shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Argument {
    // `CommandRow`'s `JsonSchema` impl in `schema.rs` describes each field.
    name: String,
    short: Option<String>,
    value_names: Vec<String>,
    required: bool,
    many: bool,
    defaults: Vec<String>,
    possible_values: Vec<String>,
    value_hint: Option<String>,
    help: Option<String>,
    long_help: Option<String>,
    global: bool,
    one_of: Vec<String>,
}

/// One example of a command, from the tool record's leaf for the command.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandExample {
    // `CommandRow`'s `JsonSchema` impl in `schema.rs` describes each field.
    command: String,
    about: &'static str,
}

/// One column of the result that a command prints, from the document of
/// the tool record's leaf for the command.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandColumn {
    // `CommandRow`'s `JsonSchema` impl in `schema.rs` describes each field.
    name: &'static str,
    long: bool,
    description: Option<String>,
}

/// One goal heading of the command's help page and the subcommands that the
/// page lists under it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandGoal {
    // `CommandRow`'s `JsonSchema` impl in `schema.rs` describes each field.
    heading: &'static str,
    commands: Vec<String>,
}

/// One exit code that the tool record declares for the command.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandExit {
    // `CommandRow`'s `JsonSchema` impl in `schema.rs` describes each field.
    exit: u8,
    last: u8,
    meaning: &'static str,
}

impl CommandRow {
    /// Returns the row of `command`, whose path from the root of `tool`'s
    /// tree is `path`, the real names of its subcommands. `parent` is the
    /// command above it, or `None` for the first command of the branch,
    /// whose row then lists every global argument it has, including those
    /// it inherits. `tool` holds the leaf whose examples and columns the
    /// row lists, the exit codes that cover the command and the group whose
    /// goal headings the row lists. `built_in` says whether crumbkit
    /// provides the command, as [`built_in`] decides.
    ///
    /// The row reads only the command's name, about texts and arguments,
    /// and the tool record. It never reads a command's help template,
    /// `before_help` or `after_help`, where `crumbkit::parse` adds its help
    /// lines on the walked path, and it reads `Arg::is_hide_set`, which
    /// `parse` never changes, so a row is the same whichever path the
    /// caller typed.
    pub(super) fn new(
        tool_name: &str,
        path: &[&str],
        command: &Command,
        parent: Option<&Command>,
        tool: &Tool,
        built_in: bool,
    ) -> Self {
        let line = std::iter::once(tool_name)
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        let leaf = tool
            .get_help()
            .get_leaves()
            .iter()
            .find(|leaf| leaf.path() == path);
        let document = leaf.and_then(Leaf::get_document);
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
        let columns = document.map_or_else(Vec::new, |document| {
            document
                .column_descriptions()
                .into_iter()
                .map(|column| CommandColumn {
                    name: column.name(),
                    long: column.is_long(),
                    description: column.description().map(ToOwned::to_owned),
                })
                .collect()
        });
        let exits = tool
            .get_failures()
            .get_exits()
            .iter()
            .filter(|exit| exit.covers(path))
            .map(|exit| CommandExit {
                exit: *exit.codes().start(),
                last: *exit.codes().end(),
                meaning: exit.meaning(),
            })
            .collect();
        // `check_help` rejects two groups with one path. `rfind` follows
        // help's rule that the last such group's headings show.
        let group = tool
            .get_help()
            .get_groups()
            .iter()
            .rfind(|group| group.path() == path);
        let goals = group.map_or_else(Vec::new, |group| {
            goal_sections(command, group.goals())
                .into_iter()
                .map(|(heading, commands)| CommandGoal {
                    heading,
                    commands: commands
                        .iter()
                        .map(|command| command.get_name().to_owned())
                        .collect(),
                })
                .collect()
        });
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
            line: document.and_then(LeafDocument::line),
            children: document
                .and_then(LeafDocument::children)
                .map(Children::get_field),
            children_help: document
                .and_then(LeafDocument::children)
                .map(Children::text),
            exits,
            effects: leaf.and_then(Leaf::get_effects),
            writes: effects::guards(leaf.and_then(Leaf::get_effects), takes_expect(command)),
            goals,
            prints_no_result: leaf.is_some_and(Leaf::get_prints_no_result),
            built_in,
        }
    }
}

/// Reports whether `command`, a leaf at `path` below the root of `tool`'s
/// tree, is one that crumbkit or one of its add-on crates provides: its
/// leaf in `tool` declares a document or stream that
/// [`Document::built_in`](crate::Document::built_in) marks. A command
/// without a leaf, or whose leaf declares no document, is not.
///
/// A group's mark comes from the commands below it, which
/// [`list`](super::list) works out as it visits them.
pub(super) fn built_in(path: &[&str], tool: &Tool) -> bool {
    tool.get_help()
        .get_leaves()
        .iter()
        .find(|leaf| leaf.path() == path)
        .and_then(Leaf::get_document)
        .is_some_and(|document| document.document().is_built_in())
}

/// Returns the form of the document that `leaf` declares, as `commands
/// list` writes it: `stream`, `bytes`, `headed`, `document`,
/// or `None` when the leaf declares no document.
fn form(leaf: &Leaf) -> Option<&'static str> {
    let document = leaf.get_document()?;
    Some(if document.is_stream() {
        "stream"
    } else if document.shape() == Some(Shape::Bytes) {
        "bytes"
    } else if document.document().is_headed() {
        "headed"
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
            value_hint: value_hint(arg.get_value_hint()),
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

/// Returns the kebab-case name of `hint`, such as `any-path`, or `None` for
/// `ValueHint::Unknown`, which is clap's report that an argument has no hint.
///
/// clap reports `AnyPath` for an argument that parses to a `PathBuf` and has
/// no hint of its own, so such an argument needs no attribute to be known as
/// a path.
//
// RUST-10: this reads the name from `ValueHint`'s `Debug` output, because
// the enum is `#[non_exhaustive]` and has no method that returns its name. A
// `match` would need a wildcard arm that reports a hint from a later clap as
// no hint. `Debug` output is not a promise from clap, so the tests list the
// name of every hint and fail if it changes. A name method on `ValueHint`
// replaces it.
fn value_hint(hint: ValueHint) -> Option<String> {
    if hint == ValueHint::Unknown {
        return None;
    }
    let mut name = String::new();
    for letter in format!("{hint:?}").chars() {
        if letter.is_ascii_uppercase() && !name.is_empty() {
            name.push('-');
        }
        name.push(letter.to_ascii_lowercase());
    }
    Some(name)
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
/// missing text. The exceptions are `goals`, which a row whose help page
/// prints no goal headings leaves out, and `prints_no_result`, which only
/// the row of a command that prints no result has, as `true`.
///
/// The `Serialize` impls of this module are written by hand so crumbkit
/// needs no `serde_derive`.
impl Serialize for CommandRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let fields = 14 + usize::from(!self.goals.is_empty()) + usize::from(self.prints_no_result);
        let mut row = serializer.serialize_struct("CommandRow", fields)?;
        row.serialize_field("path", &self.path)?;
        row.serialize_field("about", &self.about)?;
        row.serialize_field("long_about", &self.long_about)?;
        row.serialize_field("arguments", &self.arguments)?;
        row.serialize_field("examples", &self.examples)?;
        row.serialize_field("columns", &self.columns)?;
        row.serialize_field("form", &self.form)?;
        row.serialize_field("line", &self.line)?;
        row.serialize_field("children", &self.children)?;
        row.serialize_field("children_help", &self.children_help)?;
        row.serialize_field("exits", &self.exits)?;
        row.serialize_field("effects", &self.effects)?;
        row.serialize_field("writes", &self.writes)?;
        row.serialize_field("built_in", &self.built_in)?;
        if !self.goals.is_empty() {
            row.serialize_field("goals", &self.goals)?;
        }
        if self.prints_no_result {
            row.serialize_field("prints_no_result", &true)?;
        }
        row.end()
    }
}

impl Serialize for Argument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut argument = serializer.serialize_struct("Argument", 12)?;
        argument.serialize_field("name", &self.name)?;
        argument.serialize_field("short", &self.short)?;
        argument.serialize_field("value_names", &self.value_names)?;
        argument.serialize_field("required", &self.required)?;
        argument.serialize_field("many", &self.many)?;
        argument.serialize_field("defaults", &self.defaults)?;
        argument.serialize_field("possible_values", &self.possible_values)?;
        argument.serialize_field("value_hint", &self.value_hint)?;
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

impl Serialize for CommandGoal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut goal = serializer.serialize_struct("CommandGoal", 2)?;
        goal.serialize_field("heading", &self.heading)?;
        goal.serialize_field("commands", &self.commands)?;
        goal.end()
    }
}

impl Serialize for CommandExit {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut exit = serializer.serialize_struct("CommandExit", 3)?;
        exit.serialize_field("exit", &self.exit)?;
        exit.serialize_field("last", &self.last)?;
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
