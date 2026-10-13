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
/// to show by their clap ids, each as a placeholder such as `<ID>...` or with
/// a concrete value. crumbkit's `crumbkit::RenderCall::render` turns them
/// into text against the `clap::Command` that the tool built at runtime, so
/// a call that names a hidden or removed command never prints.
///
/// ```
/// use crumbkit_core::{Call, CallArg, CallTarget};
///
/// let call = Call::new(&["books", "list"]).value("author", "Le Guin");
///
/// let CallTarget::Command { path, args } = call.target() else {
///     unreachable!("Call::new names a command");
/// };
/// assert_eq!(*path, ["books", "list"]);
/// assert_eq!(
///     *args,
///     [CallArg::Value { id: "author", value: "Le Guin".to_owned() }]
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    /// The command line the call names.
    target: CallTarget,
}

/// The three forms of a [`Call`]: a command with arguments that a tool
/// names, the `-h` of a command that the caller typed, and a command line as
/// the caller typed it.
///
/// [`Call::target`] returns it, so crumbkit can render the call. The enum
/// has no `#[non_exhaustive]`, because crumbkit renders every form, and a
/// new form needs a new rendering in the same release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallTarget {
    /// A command that [`Call::new`] names, with the arguments that
    /// [`Call::arg`] and [`Call::value`] add.
    Command {
        /// The names of the subcommands from the root to the command, in
        /// order.
        path: &'static [&'static str],
        /// The arguments to show, in the order they were added.
        args: Vec<CallArg>,
    },
    /// The `-h` of a command, which [`Call::help`] names. It holds the names
    /// of the subcommands from the root to the command, as the caller typed
    /// them and the tree resolved them.
    Help(Vec<String>),
    /// A command line as the caller typed it, which [`Call::typed`] names. It
    /// holds the words after the program name, in order.
    Typed(Vec<String>),
}

/// One argument of a [`CallTarget::Command`], named by its clap id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallArg {
    /// The argument as clap's usage line shows it, such as `<ID>...` or
    /// `--author <AUTHOR>`, which [`Call::arg`] adds.
    Placeholder(&'static str),
    /// The argument with a concrete value, such as `--author 'Le Guin'`,
    /// which [`Call::value`] adds.
    Value {
        /// The clap id of the argument.
        id: &'static str,
        /// The value, as the caller types it before a shell quotes it.
        value: String,
    },
    /// The argument with every value of a list field of the failure that
    /// offers the call, which [`Call::all`] adds. It renders as its
    /// placeholder, such as `<ID>...`.
    All {
        /// The clap id of the argument.
        id: &'static str,
        /// The name of the failure's field whose values the argument takes.
        field: &'static str,
    },
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
    /// takes no arguments, so [`Call::arg`] and [`Call::value`] leave it
    /// unchanged.
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

    /// Returns a call of the command line whose words after the program
    /// name are `words`, such as `books remove 2 --yes`.
    ///
    /// `crumbkit::Output::confirm` uses it as the recovery of a run that
    /// needs `--yes`, because the caller's own words, with `--yes` added,
    /// are the command to run next. The call renders each word without
    /// looking it up, and quotes a word that a POSIX shell would split or
    /// expand. A typed call takes no arguments, so [`Call::arg`] and
    /// [`Call::value`] leave it unchanged.
    ///
    /// ```
    /// use crumbkit_core::{Call, CallTarget};
    ///
    /// let words = vec!["books".to_owned(), "remove".to_owned(), "--yes".to_owned()];
    /// let call = Call::typed(words.clone());
    ///
    /// assert_eq!(call.target(), &CallTarget::Typed(words));
    /// ```
    #[must_use]
    pub const fn typed(words: Vec<String>) -> Self {
        Self {
            target: CallTarget::Typed(words),
        }
    }

    /// Adds the argument whose clap id is `id`, such as `ids` for a field
    /// named `ids`, after the arguments already added. It renders as clap's
    /// usage line shows it, such as `<ID>...`.
    #[must_use]
    pub fn arg(mut self, id: &'static str) -> Self {
        if let CallTarget::Command { args, .. } = &mut self.target {
            args.push(CallArg::Placeholder(id));
        }
        self
    }

    /// Adds the argument whose clap id is `id` with the concrete value
    /// `value`, after the arguments already added, so the caller can run the
    /// command as it prints.
    ///
    /// An option renders as `--author 'Le Guin'`, with its short name when
    /// it has no long name, and a positional renders as the value alone. A
    /// value that a POSIX shell would split or expand renders between single
    /// quotes, as a leaf's examples do. Each call adds one occurrence, so two
    /// values of one positional render as two words, and two values of one
    /// option repeat the option. crumbkit's `RenderCall::render` says which
    /// values it refuses.
    ///
    /// ```
    /// use crumbkit_core::{Call, CallArg, CallTarget};
    ///
    /// let call = Call::new(&["books", "show"]).value("ids", "7");
    ///
    /// let CallTarget::Command { args, .. } = call.target() else {
    ///     unreachable!("Call::new names a command");
    /// };
    /// assert_eq!(*args, [CallArg::Value { id: "ids", value: "7".to_owned() }]);
    /// ```
    #[must_use]
    pub fn value(mut self, id: &'static str, value: impl Into<String>) -> Self {
        if let CallTarget::Command { args, .. } = &mut self.target {
            args.push(CallArg::Value {
                id,
                value: value.into(),
            });
        }
        self
    }

    /// Adds the argument whose clap id is `id`, to be given every value of
    /// the list field `field` of the failure that offers the call, after the
    /// arguments already added.
    ///
    /// The argument must take many values, and it renders as its
    /// placeholder, such as `<ID>...` for a positional or `--id <ID>...` for
    /// an option. A call that holds it holds no placeholder from
    /// [`Call::arg`], so the field's values alone fill the call. A failure
    /// whose recovery holds such a call names the field beside the call in
    /// its JSON, and its text prints the call with the values in place of the
    /// placeholder. A batch's over-budget failure uses it to name the command
    /// that fetches every key it left out in one run.
    ///
    /// ```
    /// use crumbkit_core::{Call, CallArg, CallTarget};
    ///
    /// let call = Call::new(&["books", "show"]).all("ids", "left_out");
    ///
    /// let CallTarget::Command { args, .. } = call.target() else {
    ///     unreachable!("Call::new names a command");
    /// };
    /// assert_eq!(*args, [CallArg::All { id: "ids", field: "left_out" }]);
    /// ```
    #[must_use]
    pub fn all(mut self, id: &'static str, field: &'static str) -> Self {
        if let CallTarget::Command { args, .. } = &mut self.target {
            args.push(CallArg::All { id, field });
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
    /// The call gives a value to the argument `id` of `command`, and the
    /// argument is a flag that takes no value.
    TakesNoValue {
        /// The final command of the call, such as `shelf books list`.
        command: String,
        /// The id of the flag.
        id: String,
    },
    /// The call gives the argument `id` of `command` every value of a field,
    /// and the argument takes one value.
    TakesOneValue {
        /// The final command of the call, such as `shelf books list`.
        command: String,
        /// The id of the argument.
        id: String,
    },
    /// The call holds the argument `id` of `command` as a placeholder beside
    /// an argument that takes every value of a field, or holds two arguments
    /// that take every value of a field. Only the values of one field fill
    /// such a call.
    PlaceholderBesideAll {
        /// The final command of the call, such as `shelf books list`.
        command: String,
        /// The id of the argument that is a placeholder.
        id: String,
    },
    /// The call gives the argument `id` of `command` a value that is not one
    /// of the argument's possible values.
    UnknownValue {
        /// The final command of the call, such as `shelf books list`.
        command: String,
        /// The id of the argument.
        id: String,
        /// The value that the argument does not accept.
        value: String,
    },
    /// The call gives the positional argument `id` of `command` a value that
    /// starts with `-`, which clap reads as a flag, and the argument does not
    /// allow such values.
    HyphenValue {
        /// The final command of the call, such as `shelf books show`.
        command: String,
        /// The id of the positional argument.
        id: String,
        /// The value that starts with `-`.
        value: String,
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
            Self::TakesNoValue { command, id } => {
                write!(f, "'{command}' takes no value for its argument '{id}'")
            }
            Self::TakesOneValue { command, id } => write!(
                f,
                "'{command}' takes one value for its argument '{id}', so a call cannot give it every value of a field"
            ),
            Self::PlaceholderBesideAll { command, id } => write!(
                f,
                "'{command}' has its argument '{id}' as a placeholder in a call that takes every value of a field, which only those values fill"
            ),
            Self::UnknownValue { command, id, value } => {
                write!(
                    f,
                    "'{command}' does not accept '{value}' for its argument '{id}'"
                )
            }
            Self::HyphenValue { command, id, value } => write!(
                f,
                "'{command}' would read the value '{value}' of its argument '{id}' as a flag"
            ),
        }
    }
}

impl Error for CallError {}
