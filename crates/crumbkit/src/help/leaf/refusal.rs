//! The calls that a refusal of a leaf's command line offers as its
//! recovery and alternatives, which a function of the tool builds from the
//! refusal, set on the leaf or on the guide for every leaf that sets none
//! (ADR-0056, ADR-0078).

use std::ffi::OsString;

use clap::Command;
use clap::error::{ContextKind, ContextValue, ErrorKind};

use super::Leaf;
use crate::Call;
use crate::help::Guide;

#[cfg(test)]
mod tests;

/// What clap refused in a command line that names a leaf, which the
/// function from [`Leaf::refusal_alternatives`] or
/// [`Guide::refusal_alternatives`] reads to build its calls.
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
    /// line that names the command offers as its recovery and alternatives
    /// (ADR-0056, ADR-0078).
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
    /// function. crumbkit renders each returned call against the command
    /// tree, and parses the line of each call that holds no placeholder from
    /// [`Call::arg`] as the run would. It leaves out a call that does not
    /// render or whose line does not parse, without a warning, so a
    /// function can return a correction that it has not checked.
    ///
    /// The first call that crumbkit keeps becomes the refusal's recovery in
    /// place of the command's `-h`, and the `-h` call becomes the first
    /// alternative, followed by the other kept calls in the order that the
    /// function returned them. The `-h` call names the deepest command on
    /// the typed path that has a help flag, and a path without one offers
    /// no `-h` call. When crumbkit keeps no call, the `-h` call stays the
    /// recovery. A `--limit` above its bound keeps its recovery,
    /// which replaces `--limit` with `--all`, and offers the kept calls
    /// after crumbkit's own alternatives. The refusal keeps its code and its
    /// exit code.
    ///
    /// The leaf's function replaces the function of
    /// [`Guide::refusal_alternatives`] for this command line, and one that
    /// wants the guide's calls too calls the guide's function itself.
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

impl Guide {
    /// Sets the function that builds the alternatives for a refused command
    /// line whose command has no function of its own from
    /// [`Leaf::refusal_alternatives`], including a command with no leaf in
    /// the guide (ADR-0056).
    ///
    /// A tool whose correction is one rule for every command, such as
    /// rewriting a refused value and keeping the corrected line when it
    /// parses, declares it here once. crumbkit treats the calls that it
    /// returns as it treats a leaf's calls: the first call that renders and
    /// whose line parses becomes the refusal's recovery in place of `-h`,
    /// and the `-h` call becomes the first alternative. A leaf's own
    /// function replaces this one for the leaf's command line:
    ///
    /// ```
    /// use crumbkit::Call;
    /// use crumbkit::help::{Guide, RefusalContext};
    ///
    /// /// Offers the refused line with `.json` cut from the refused value.
    /// fn without_json(context: &RefusalContext<'_>) -> Vec<Call> {
    ///     let Some(value) = context.value() else {
    ///         return Vec::new();
    ///     };
    ///     let Some(stem) = value.strip_suffix(".json") else {
    ///         return Vec::new();
    ///     };
    ///     let words = context.words().iter().map(|word| {
    ///         let word = word.to_string_lossy();
    ///         if word == value { stem.to_owned() } else { word.into_owned() }
    ///     });
    ///     vec![Call::typed(words.collect())]
    /// }
    ///
    /// const GUIDE: Guide = Guide::new().refusal_alternatives(without_json);
    /// # assert!(GUIDE.get_refusal_alternatives().is_some());
    /// ```
    #[must_use]
    pub const fn refusal_alternatives(
        mut self,
        alternatives: fn(&RefusalContext<'_>) -> Vec<Call>,
    ) -> Self {
        self.refusal_alternatives = Some(alternatives);
        self
    }

    /// Returns the function that [`Guide::refusal_alternatives`] set, or
    /// `None` when a command without a function of its own offers
    /// crumbkit's alternatives alone. The name has clap's `get_` prefix,
    /// because `refusal_alternatives` is the builder method.
    #[must_use]
    pub const fn get_refusal_alternatives(&self) -> Option<fn(&RefusalContext<'_>) -> Vec<Call>> {
        self.refusal_alternatives
    }

    /// Returns the function that builds the alternatives for a refused
    /// command line that names the command at `path`: the function of the
    /// command's leaf, or else the guide's, or `None` when neither is set.
    pub(crate) fn refusal_function(
        &self,
        path: &[String],
    ) -> Option<fn(&RefusalContext<'_>) -> Vec<Call>> {
        self.leaves
            .iter()
            .find(|leaf| leaf.path() == path)
            .and_then(Leaf::get_refusal_alternatives)
            .or(self.refusal_alternatives)
    }
}
