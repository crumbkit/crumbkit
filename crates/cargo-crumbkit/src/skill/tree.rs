//! A tool's command tree, read from its `commands list --all --json --full`
//! document, which `skill check` checks command lines against.

use serde::Deserialize;

use crate::tool::names;

#[cfg(test)]
mod tests;

/// Every visible command of a tool with its arguments.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Tree {
    /// The root command.
    root: Command,
    /// The commands below the root in depth-first order.
    below: Vec<Command>,
}

/// One visible command of the tool and the arguments that its row lists.
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Command {
    /// The command line that runs the command, such as `shelf books list`.
    pub(crate) path: String,
    /// The command's about text, or `None` when it has none.
    #[serde(default)]
    pub(crate) about: Option<String>,
    /// The command's arguments. The root's row lists every global argument,
    /// and a lower row lists only the global arguments that its command
    /// defines.
    pub(crate) arguments: Vec<Argument>,
    /// The goal headings of the command's help page, which only the row of
    /// a command whose help prints them has.
    #[serde(default)]
    pub(crate) goals: Vec<Goal>,
}

/// One goal heading of a command's help page and the subcommands that the
/// page lists under it.
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Goal {
    /// The heading, such as `Browse the catalogue`.
    pub(crate) heading: String,
    /// The names of the subcommands under the heading, such as `books`.
    pub(crate) commands: Vec<String>,
}

/// One argument of a command, with the fields of a `commands list` row that
/// `skill check` reads. serde ignores the other fields.
#[derive(Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Argument {
    /// The argument's name as help writes it: `--all`, `-a` for one with
    /// only a short name, or `<ID>` for a positional argument.
    pub(crate) name: String,
    /// The short name, such as `-a`, or `None`.
    pub(crate) short: Option<String>,
    /// The names of the values that one use of the argument takes, which is
    /// empty for a flag that takes no value.
    pub(crate) value_names: Vec<String>,
    /// Whether the argument accepts more than one value, or repeats.
    pub(crate) many: bool,
    /// The values that the argument accepts, or empty when it accepts any.
    pub(crate) possible_values: Vec<String>,
    /// Whether the argument applies to every command below its own.
    pub(crate) global: bool,
}

/// The `<tool>/commands-list/v1` document, with only the fields that the
/// tree reads.
#[derive(Deserialize)]
struct Listed {
    /// The document's id.
    schema: String,
    /// One row for each visible command.
    data: Vec<Command>,
}

impl Tree {
    /// Returns the tree of `root` and the commands `below` it.
    #[cfg(test)]
    pub(crate) const fn new(root: Command, below: Vec<Command>) -> Self {
        Self { root, below }
    }

    /// Returns the tree that `stdout`, the output of `commands list --all
    /// --json --full`, lists, or `None` when it is not a
    /// `<tool>/commands-list/v1` document or lists no command.
    pub(crate) fn read(stdout: &[u8]) -> Option<Self> {
        let document: Listed = serde_json::from_slice(stdout).ok()?;
        if !names(&document.schema, &crumbkit::commands::COMMANDS_LIST) {
            return None;
        }
        let mut commands = document.data.into_iter();
        let root = commands.next()?;
        Some(Self {
            root,
            below: commands.collect(),
        })
    }

    /// Returns the root command.
    pub(crate) const fn root(&self) -> &Command {
        &self.root
    }

    /// Returns the name that the root command runs as, such as `shelf`.
    pub(crate) fn name(&self) -> &str {
        &self.root.path
    }

    /// Returns the command whose command line is `path`, such as `shelf
    /// books list`.
    pub(crate) fn command(&self, path: &str) -> Option<&Command> {
        self.commands().find(|command| command.path == path)
    }

    /// Returns every command, the root first and then the commands below it
    /// in depth-first order.
    pub(crate) fn commands(&self) -> impl Iterator<Item = &Command> {
        std::iter::once(&self.root).chain(&self.below)
    }

    /// Returns the flag called `name`, by its long or short name, that the
    /// command whose command line is `path` accepts: one of its own, or a
    /// global flag of a command above it. It returns `None` when `path`
    /// names no command or the command accepts no such flag.
    pub(crate) fn flag(&self, path: &str, name: &str) -> Option<&Argument> {
        let own = self.command(path)?;
        own.arguments
            .iter()
            .find(|argument| argument.is_called(name))
            .or_else(|| {
                self.commands()
                    .filter(|command| {
                        path.strip_prefix(command.path.as_str())
                            .is_some_and(|rest| rest.starts_with(' '))
                    })
                    .flat_map(|command| &command.arguments)
                    .find(|argument| argument.global && argument.is_called(name))
            })
    }

    /// Reports whether the command whose command line is `path` has a
    /// visible subcommand.
    pub(crate) fn has_subcommands(&self, path: &str) -> bool {
        self.below.iter().any(|command| {
            command
                .path
                .strip_prefix(path)
                .and_then(|rest| rest.strip_prefix(' '))
                .is_some_and(|rest| !rest.contains(' '))
        })
    }
}

impl Command {
    /// Returns the command's positional arguments in order.
    pub(crate) fn positionals(&self) -> impl Iterator<Item = &Argument> {
        self.arguments.iter().filter(|argument| !argument.is_flag())
    }
}

impl Argument {
    /// Reports whether the argument is a flag, with a name that starts with
    /// `-`, rather than a positional argument.
    pub(crate) fn is_flag(&self) -> bool {
        self.name.starts_with('-')
    }

    /// Reports whether the argument is called `name`, its long name such as
    /// `--all` or its short name such as `-a`.
    pub(crate) fn is_called(&self, name: &str) -> bool {
        self.is_flag() && (self.name == name || self.short.as_deref() == Some(name))
    }
}
