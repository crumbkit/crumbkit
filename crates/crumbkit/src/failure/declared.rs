//! The record of what a tool declares about its failures: its exit codes
//! and its answers to a refused command line.

use crate::Call;
use crate::tool::{Exit, RefusalContext};

/// What a tool declares about its failures: its exit codes besides
/// crumbkit's 0, 1 and 2, and the functions that answer a refused command
/// line.
///
/// [`Tool::failures`](crate::Tool::failures) takes it. A tool that sets
/// nothing here exits with crumbkit's codes.
///
/// ```
/// use crumbkit::Tool;
/// use crumbkit::tool::{Exit, Failures};
///
/// const TOOL: Tool =
///     Tool::new().failures(Failures::new().exits(&[Exit::new(3, "The shelf was locked.")]));
/// assert_eq!(TOOL.get_failures().get_exits().len(), 1);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Failures {
    /// The tool's own exit codes, which `<tool> help exit-codes` lists.
    exits: &'static [Exit],
    /// The function that builds refusal alternatives for a command whose
    /// leaf sets none, or `None` for crumbkit's alternatives alone.
    refusal_alternatives: Option<fn(&RefusalContext<'_>) -> Vec<Call>>,
    /// The function that returns the refusal hint for a command whose leaf
    /// sets none, or `None` for no hint.
    refusal_hint: Option<fn(&RefusalContext<'_>) -> Option<String>>,
}

impl Failures {
    /// Returns the record with no exit codes of the tool's own and no
    /// refusal functions.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            exits: &[],
            refusal_alternatives: None,
            refusal_hint: None,
        }
    }

    /// Sets the exit codes that the tool returns besides crumbkit's 0, 1
    /// and 2, in the order that `<tool> help exit-codes` lists them. An
    /// exit can list the tool's failure codes and crumbkit's, so a tool
    /// gives a refused command line its own number, such as 64, by listing
    /// `crumbkit::codes::CLI_USAGE` (ADR-0056).
    #[must_use]
    pub const fn exits(mut self, exits: &'static [Exit]) -> Self {
        self.exits = exits;
        self
    }

    /// Returns the exit codes that
    /// [`Failures::exits`](crate::tool::Failures::exits) set. The name has
    /// clap's `get_` prefix, because `exits` is the builder method.
    #[must_use]
    pub const fn get_exits(&self) -> &'static [Exit] {
        self.exits
    }

    /// Sets the function that builds the alternatives for a refused command
    /// line whose command has no function of its own from
    /// [`Leaf::refusal_alternatives`](crate::help::Leaf::refusal_alternatives), including a command with no leaf in
    /// the tool record (ADR-0056).
    ///
    /// A tool whose correction is one rule for every command, such as rewriting
    /// a refused value and keeping the corrected line when it parses, declares
    /// it here once. crumbkit treats the calls that it returns as it treats a
    /// leaf's calls: the first call that renders and whose line parses becomes
    /// the refusal's recovery in place of `-h`, unless a hint from
    /// [`Failures::refusal_hint`](crate::tool::Failures::refusal_hint) or
    /// [`Leaf::refusal_hint`](crate::help::Leaf::refusal_hint) takes it, and the `-h` call becomes the first
    /// alternative. A leaf's own function replaces this one for the leaf's
    /// command line:
    ///
    /// ```
    /// use crumbkit::Call;
    /// use crumbkit::Tool;
    /// use crumbkit::tool::{Failures, RefusalContext};
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
    /// const TOOL: Tool = Tool::new().failures(Failures::new().refusal_alternatives(without_json));
    /// # assert!(TOOL.get_failures().get_refusal_alternatives().is_some());
    /// ```
    #[must_use]
    pub const fn refusal_alternatives(
        mut self,
        alternatives: fn(&RefusalContext<'_>) -> Vec<Call>,
    ) -> Self {
        self.refusal_alternatives = Some(alternatives);
        self
    }

    /// Returns the function that
    /// [`Failures::refusal_alternatives`](crate::tool::Failures::refusal_alternatives)
    /// set, or `None` when a command without a function of its own offers
    /// crumbkit's alternatives alone. The name has clap's `get_` prefix,
    /// because `refusal_alternatives` is the builder method.
    #[must_use]
    pub const fn get_refusal_alternatives(&self) -> Option<fn(&RefusalContext<'_>) -> Vec<Call>> {
        self.refusal_alternatives
    }

    /// Sets the function that returns the hint for a refused command line
    /// whose command has no function of its own from
    /// [`Leaf::refusal_hint`](crate::help::Leaf::refusal_hint), including a command with no leaf in the
    /// tool record (ADR-0123).
    ///
    /// A tool whose instruction is one rule for every command, such as
    /// telling the caller to put an unknown flag after `--` at any command
    /// that passes the words after `--` to a program, declares it here
    /// once. crumbkit treats the sentence that it returns as it treats a
    /// leaf's: the sentence becomes the refusal's recovery, and the `-h`
    /// call becomes the first alternative. A leaf's own function replaces
    /// this one for the leaf's command line:
    ///
    /// ```
    /// use clap::Arg;
    /// use clap::error::ErrorKind;
    /// use crumbkit::Tool;
    /// use crumbkit::tool::{Failures, RefusalContext};
    ///
    /// /// Tells the caller where the program's words go.
    /// fn after_dashes(context: &RefusalContext<'_>) -> Option<String> {
    ///     let forwards = context.command().get_positionals().any(Arg::is_last_set);
    ///     (forwards && context.kind() == ErrorKind::UnknownArgument).then(|| {
    ///         "put everything the tool should pass to the program it runs after `--`"
    ///             .to_owned()
    ///     })
    /// }
    ///
    /// const TOOL: Tool = Tool::new().failures(Failures::new().refusal_hint(after_dashes));
    /// # assert!(TOOL.get_failures().get_refusal_hint().is_some());
    /// ```
    #[must_use]
    pub const fn refusal_hint(mut self, hint: fn(&RefusalContext<'_>) -> Option<String>) -> Self {
        self.refusal_hint = Some(hint);
        self
    }

    /// Returns the function that
    /// [`Failures::refusal_hint`](crate::tool::Failures::refusal_hint) set, or
    /// `None` when a command without a function of its own offers no hint. The
    /// name has clap's `get_` prefix, because `refusal_hint` is the builder
    /// method.
    #[must_use]
    pub const fn get_refusal_hint(&self) -> Option<fn(&RefusalContext<'_>) -> Option<String>> {
        self.refusal_hint
    }
}
