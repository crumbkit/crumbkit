//! The alternatives that a refusal of a leaf's command line offers after
//! crumbkit's own, which a function of the tool builds from the refusal
//! (ADR-0056).

use std::ffi::OsString;

use clap::Command;
use clap::error::{ContextKind, ContextValue, ErrorKind};

use super::Leaf;
use crate::Call;

#[cfg(test)]
mod tests;

/// What clap refused in a command line that names a leaf, which the leaf's
/// function from [`Leaf::refusal_alternatives`] reads to build its calls.
///
/// The fields are private so that a later release can add more of the
/// refusal without breaking a tool's function.
#[derive(Debug)]
pub struct RefusalContext<'a> {
    /// The clap id of the refused argument, or `None` when clap names no
    /// argument of the leaf's command.
    argument: Option<&'a str>,
    /// The refused value, or `None` when clap names none.
    value: Option<&'a str>,
    /// The kind of clap's error.
    kind: ErrorKind,
    /// The words the caller typed after the program name.
    words: &'a [OsString],
}

impl<'a> RefusalContext<'a> {
    /// Returns the context of `error`, with which clap refused `words`, the
    /// caller's words after the program name, at `command`, the leaf's
    /// command after a build.
    ///
    /// clap names the refused argument as its usage line shows it, such as
    /// `<ID>` or `--sort <KEY>`, so the argument is the one of `command`
    /// that displays as that text.
    pub(crate) fn new(command: &'a Command, error: &'a clap::Error, words: &'a [OsString]) -> Self {
        let shown = match error.get(ContextKind::InvalidArg) {
            Some(ContextValue::String(text)) => Some(text.as_str()),
            _ => None,
        };
        let argument = shown.and_then(|shown| {
            command
                .get_arguments()
                .find(|arg| arg.to_string() == shown)
                .map(|arg| arg.get_id().as_str())
        });
        let value = match error.get(ContextKind::InvalidValue) {
            Some(ContextValue::String(value)) => Some(value.as_str()),
            _ => None,
        };
        Self {
            argument,
            value,
            kind: error.kind(),
            words,
        }
    }

    /// Returns the clap id of the refused argument, such as `id` for a
    /// field named `id`, or `None` when clap names no argument of the
    /// leaf's command, as for an unknown flag.
    #[must_use]
    pub const fn argument(&self) -> Option<&'a str> {
        self.argument
    }

    /// Returns the value that clap refused, such as `a.json` for a
    /// positional that takes a number, or `None` when clap names no value.
    #[must_use]
    pub const fn value(&self) -> Option<&'a str> {
        self.value
    }

    /// Returns the kind of clap's error, such as
    /// `ErrorKind::ValueValidation` for a value that the argument's parser
    /// refused.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Returns the words the caller typed after the program name, in order,
    /// which a function rewrites into a corrected line with
    /// [`Call::typed`].
    #[must_use]
    pub const fn words(&self) -> &'a [OsString] {
        self.words
    }
}

impl Leaf {
    /// Sets the function that builds the calls that a refusal of a command
    /// line that names the command offers as alternatives, after the
    /// alternatives that crumbkit works out itself (ADR-0056).
    ///
    /// A tool that knows a better next step than crumbkit's declares it
    /// here, such as listing the ids that `books show` takes, or the
    /// refused line with the refused value corrected:
    ///
    /// ```
    /// use crumbkit::Call;
    /// use crumbkit::help::{Leaf, RefusalContext};
    ///
    /// /// Offers `get /tasks/a` for a refused `get tasks/a.json`.
    /// fn as_path(context: &RefusalContext<'_>) -> Vec<Call> {
    ///     let Some(value) = context.value() else {
    ///         return Vec::new();
    ///     };
    ///     let stem = value.strip_suffix(".json").unwrap_or(value);
    ///     vec![Call::new(&["get"]).value("path", format!("/{stem}"))]
    /// }
    ///
    /// const GET: Leaf = Leaf::new(&["get"]).refusal_alternatives(as_path);
    /// # assert!(GET.get_refusal_alternatives().is_some());
    /// ```
    ///
    /// A command line names the command when its words reach it, so a
    /// misspelt subcommand below its parent names the parent and calls no
    /// function. The refusal keeps its recovery, its code and its exit code.
    /// crumbkit renders each returned call against the command tree, and
    /// parses the line of each call that holds no placeholder from
    /// [`Call::arg`] as the run would. It leaves out a call that does not
    /// render or whose line does not parse, without a warning, so a
    /// function can return a correction that it has not checked.
    #[must_use]
    pub const fn refusal_alternatives(
        mut self,
        alternatives: fn(&RefusalContext<'_>) -> Vec<Call>,
    ) -> Self {
        self.refusal_alternatives = Some(alternatives);
        self
    }

    /// Returns the function that [`Leaf::refusal_alternatives`] set, or
    /// `None` when the leaf offers crumbkit's alternatives alone. The name
    /// has clap's `get_` prefix, because `refusal_alternatives` is the
    /// builder method.
    #[must_use]
    pub const fn get_refusal_alternatives(&self) -> Option<fn(&RefusalContext<'_>) -> Vec<Call>> {
        self.refusal_alternatives
    }
}
