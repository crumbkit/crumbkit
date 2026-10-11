//! A command line that names a command and its arguments, which crumbkit
//! checks against the command tree a tool builds at runtime.

use std::error::Error;
use std::fmt;

#[cfg(test)]
mod tests;

/// A command line that a tool's output tells the caller to run next, such as
/// `shelf books show <ID>...`.
///
/// A call names a command by its path from the root and names the arguments
/// to show by their clap ids. It holds names only. crumbkit's
/// `crumbkit::RenderCall::render` turns them into text against the
/// `clap::Command` that the tool built at runtime, so a call that names a
/// hidden or removed command never prints.
///
/// ```
/// use crumbkit_core::{Call, CallTarget};
///
/// let call = Call::new(&["books", "show"]).arg("ids");
///
/// let CallTarget::Command { path, args, .. } = call.target() else {
///     unreachable!("Call::new names a command");
/// };
/// assert_eq!(*path, ["books", "show"]);
/// assert_eq!(*args, ["ids"]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// The command line the call names.
    target: CallTarget,
}

/// The two forms of a [`Call`]: a command with arguments that a tool names
/// at compile time and words added at runtime, and the `-h` of a command
/// that the caller typed.
///
/// [`Call::target`] returns it, so crumbkit can render the call. The enum
/// has no `#[non_exhaustive]`, because crumbkit renders every form, and a
/// new form needs a new rendering in the same release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallTarget {
    /// A command that [`Call::new`] and [`Call::arg`] name, followed by the
    /// words of [`Call::word`].
    Command {
        /// The names of the subcommands from the root to the command, in
        /// order.
        path: &'static [&'static str],
        /// The clap ids of the arguments to show, in order.
        args: Vec<&'static str>,
        /// The words to print after the arguments, in order, such as the
        /// command names that `commands list` takes.
        words: Vec<String>,
    },
    /// The `-h` of a command, which [`Call::help`] names. It holds the names
    /// of the subcommands from the root to the command, as the caller typed
    /// them and the tree resolved them.
    Help(Vec<String>),
}

impl Call {
    /// Returns a call of the command at `path`, the names of its subcommands
    /// from the root, with no arguments. An empty path names the root.
    #[must_use]
    pub const fn new(path: &'static [&'static str]) -> Self {
        Self {
            target: CallTarget::Command {
                path,
                args: Vec::new(),
                words: Vec::new(),
            },
        }
    }

    /// Returns a call that runs `-h` on the command at `path`, such as
    /// `shelf books show -h`.
    ///
    /// crumbkit's refusal of a command line uses it as its recovery, because
    /// the path comes from the words the caller typed at runtime, and a
    /// `Call` built with [`Call::new`] holds only a `&'static` path. The
    /// call renders its path without looking it up, so it can point the
    /// caller back at a hidden command that the caller named. A help call
    /// takes no arguments and no words, so [`Call::arg`] and [`Call::word`]
    /// leave it unchanged.
    ///
    /// ```
    /// use crumbkit_core::{Call, CallTarget};
    ///
    /// let call = Call::help(vec!["books".to_owned(), "show".to_owned()]);
    ///
    /// assert_eq!(
    ///     call.target(),
    ///     &CallTarget::Help(vec!["books".to_owned(), "show".to_owned()])
    /// );
    /// ```
    #[must_use]
    pub const fn help(path: Vec<String>) -> Self {
        Self {
            target: CallTarget::Help(path),
        }
    }

    /// Adds the argument whose clap id is `id`, such as `ids` for a field
    /// named `ids`, after the arguments already added.
    #[must_use]
    pub fn arg(mut self, id: &'static str) -> Self {
        if let CallTarget::Command { args, .. } = &mut self.target {
            args.push(id);
        }
        self
    }

    /// Adds `word` after the arguments and the words already added, as a
    /// word that the caller types as it is, such as `books` in
    /// `shelf commands list books`.
    ///
    /// The recovery of crumbkit's `commands list` uses it, because its words
    /// come from the caller at runtime. A word that a POSIX shell would
    /// split or expand renders between single quotes, as a leaf's examples
    /// do.
    ///
    /// ```
    /// use crumbkit_core::{Call, CallTarget};
    ///
    /// let call = Call::new(&["commands", "list"]).word("books".to_owned());
    ///
    /// let CallTarget::Command { words, .. } = call.target() else {
    ///     unreachable!("Call::new names a command");
    /// };
    /// assert_eq!(*words, ["books"]);
    /// ```
    #[must_use]
    pub fn word(mut self, word: String) -> Self {
        if let CallTarget::Command { words, .. } = &mut self.target {
            words.push(word);
        }
        self
    }

    /// Returns the command line the call names, which crumbkit reads to
    /// render the call.
    #[must_use]
    pub const fn target(&self) -> &CallTarget {
        &self.target
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
