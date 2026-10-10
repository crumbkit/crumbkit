//! A command line that names a command and its arguments, checked against the
//! command tree a tool builds at runtime.

use std::error::Error;
use std::fmt::{self, Write as _};

use clap::Command;

use crate::help::leaf::quote;

#[cfg(test)]
mod tests;

/// A command line that a tool's output tells the caller to run next, such as
/// `shelf books show <ID>...`.
///
/// A call names a command by its path from the root and names the arguments
/// to show by their clap ids. It holds names only, and [`Call::render`] turns
/// them into text against the `clap::Command` that the tool built at runtime,
/// so a call that names a hidden or removed command never prints.
///
/// ```
/// use clap::{CommandFactory, Parser, Subcommand};
/// use crumbkit::Call;
///
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(subcommand)]
///     command: Books,
/// }
///
/// #[derive(Subcommand)]
/// enum Books {
///     /// Show the books with these ids.
///     Show {
///         #[arg(required = true, value_name = "ID")]
///         ids: Vec<u32>,
///     },
/// }
///
/// let call = Call::new(&["show"]).arg("ids");
/// assert_eq!(call.render(&mut Cli::command()).unwrap(), "shelf show <ID>...");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// The command line the call names.
    target: Target,
}

/// The two forms of a [`Call`]: a command with arguments that a tool names
/// at compile time and words that crumbkit adds at runtime, and the `-h` of
/// a command that the caller typed.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    /// A command that [`Call::new`] and [`Call::arg`] name, followed by the
    /// words of [`Call::word`].
    Command {
        /// The names of the subcommands from the root to the command, in
        /// order.
        path: &'static [&'static str],
        /// The clap ids of the arguments to show, in order.
        args: Vec<&'static str>,
        /// The words to print after the arguments, in order, such as the
        /// command names that `commands list` takes. crumbkit adds them at
        /// runtime with [`Call::word`].
        words: Vec<String>,
    },
    /// The `-h` of a command, which [`Call::help`] names for the recovery of
    /// a refused command line. It holds the names of the subcommands from
    /// the root to the command, as the caller typed them and the tree
    /// resolved them.
    Help(Vec<String>),
}

impl Call {
    /// Returns a call of the command at `path`, the names of its subcommands
    /// from the root, with no arguments. An empty path names the root.
    #[must_use]
    pub const fn new(path: &'static [&'static str]) -> Self {
        Self {
            target: Target::Command {
                path,
                args: Vec::new(),
                words: Vec::new(),
            },
        }
    }

    /// Returns a call that runs `-h` on the command at `path`, such as
    /// `shelf books show -h`. A refusal's recovery uses it, because the path
    /// comes from the words the caller typed at runtime, and a `Call` built
    /// with [`Call::new`] holds only a `&'static` path.
    pub(crate) const fn help(path: Vec<String>) -> Self {
        Self {
            target: Target::Help(path),
        }
    }

    /// Adds the argument whose clap id is `id`, such as `ids` for a field
    /// named `ids`, after the arguments already added.
    #[must_use]
    pub fn arg(mut self, id: &'static str) -> Self {
        // A call from `Call::help` takes no arguments, and only crumbkit
        // builds one, so it keeps none.
        if let Target::Command { args, .. } = &mut self.target {
            args.push(id);
        }
        self
    }

    /// Adds `word` after the arguments and the words already added, as a
    /// word that the caller types as it is, such as `books` in
    /// `shelf commands list books`. The recovery of `commands list` uses it,
    /// because its words come from the caller at runtime. A word that a
    /// POSIX shell would split or expand prints between single quotes, as
    /// a leaf's examples do.
    pub(crate) fn word(mut self, word: String) -> Self {
        // A call from `Call::help` takes no words, and only crumbkit builds
        // one, so it keeps none.
        if let Target::Command { words, .. } = &mut self.target {
            words.push(word);
        }
        self
    }

    /// Returns the call as text: the name of `command`, then the name of each
    /// subcommand on the path, then each argument in the form clap's usage
    /// line gives it, such as `<ID>...` for a positional that takes many
    /// values or `--author <AUTHOR>` for an option.
    ///
    /// `command` is the root of the tool's tree. `render` first calls
    /// `Command::build`, which clap requires before a command's arguments are
    /// read and which does nothing to a command already built. Each path
    /// segment is looked up with `Command::find_subcommand`, so an alias
    /// resolves and prints as the subcommand's name. Each argument is looked
    /// up on the final command, which after the build also holds the global
    /// arguments of the commands above it.
    ///
    /// # Errors
    ///
    /// Returns a [`CallError`] naming the first path segment or argument that
    /// is missing or hidden. A hidden command or argument counts as
    /// unresolved, because crumbkit never prints what the tool hid.
    pub fn render(&self, command: &mut Command) -> Result<String, CallError> {
        command.build();
        self.render_built(command)
    }

    /// Returns the call as text, as [`Call::render`] does, against a command
    /// that `Command::build` has already prepared.
    ///
    /// `Output::set_command` builds the command it keeps, so `finish` renders
    /// through a shared reference. `Arg`'s `Display` panics on an option of a
    /// command that was never built, because clap sets an option's number of
    /// values during the build.
    ///
    /// A call from [`Call::help`] renders its path without looking it up,
    /// because the path holds the real names of commands that the caller
    /// typed, and crumbkit points the caller back at a hidden command that
    /// the caller named.
    pub(crate) fn render_built(&self, command: &Command) -> Result<String, CallError> {
        let mut text = command.get_name().to_owned();
        let (path, args, words) = match &self.target {
            Target::Help(path) => {
                for name in path {
                    text.push(' ');
                    text.push_str(name);
                }
                text.push_str(" -h");
                return Ok(text);
            }
            Target::Command { path, args, words } => (*path, args, words),
        };
        let mut current = command;
        for &name in path {
            let Some(subcommand) = current.find_subcommand(name) else {
                return Err(CallError::MissingCommand {
                    command: text,
                    name: name.to_owned(),
                });
            };
            if subcommand.is_hide_set() {
                return Err(CallError::HiddenCommand {
                    command: text,
                    name: name.to_owned(),
                });
            }
            text.push(' ');
            text.push_str(subcommand.get_name());
            current = subcommand;
        }

        for &id in args {
            let Some(arg) = current.get_arguments().find(|arg| arg.get_id() == id) else {
                return Err(CallError::MissingArgument {
                    command: text,
                    id: id.to_owned(),
                });
            };
            if arg.is_hide_set() {
                return Err(CallError::HiddenArgument {
                    command: text,
                    id: id.to_owned(),
                });
            }
            // `Arg`'s `Display` writes the argument as clap's usage line does,
            // without styling. Writing to a `String` cannot fail.
            let _ = write!(text, " {arg}");
        }
        for word in words {
            text.push(' ');
            text.push_str(&quote(word));
        }
        Ok(text)
    }
}

/// The reason a [`Call`] does not resolve in a command tree.
///
/// Each variant names the command that was searched, as the text of the call
/// up to that point, such as `shelf books`, and the name or id that it lacks.
/// The variants say what kind of thing is missing, so a tool's test that
/// asserts its calls resolve reports which part to fix.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CallError {
    /// `command` has no subcommand called `name`.
    MissingCommand {
        /// The command that was searched, such as `shelf books`.
        command: String,
        /// The path segment that names no subcommand.
        name: String,
    },
    /// `command` has a subcommand called `name`, and the tool hid it.
    HiddenCommand {
        /// The command that was searched, such as `shelf books`.
        command: String,
        /// The path segment that names a hidden subcommand.
        name: String,
    },
    /// `command`, the final command of the call, has no argument with the id
    /// `id`.
    MissingArgument {
        /// The final command of the call, such as `shelf books show`.
        command: String,
        /// The argument id that the command lacks.
        id: String,
    },
    /// `command`, the final command of the call, has an argument with the id
    /// `id`, and the tool hid it.
    HiddenArgument {
        /// The final command of the call, such as `shelf books show`.
        command: String,
        /// The id of the hidden argument.
        id: String,
    },
}

/// Prints the reason on one line, such as `'shelf books' has no subcommand
/// 'shw'`.
impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand { command, name } => {
                write!(f, "'{command}' has no subcommand '{name}'")
            }
            Self::HiddenCommand { command, name } => {
                write!(f, "'{command}' hides its subcommand '{name}'")
            }
            Self::MissingArgument { command, id } => {
                write!(f, "'{command}' has no argument '{id}'")
            }
            Self::HiddenArgument { command, id } => {
                write!(f, "'{command}' hides its argument '{id}'")
            }
        }
    }
}

impl Error for CallError {}
