//! The record of what a tool declares to crumbkit. [`Tool`] composes the
//! [`Help`], [`Failures`] and [`Output`] records, and this module also holds
//! the paths of the last two, with [`Exit`] and [`RefusalContext`], which
//! the failure declarations name.
//! `crumbkit::Output` names the handle of one run, so the record of what a
//! run prints has its path here.

use clap::Command;

pub use crate::failure::declared::Failures;
pub use crate::help::exit::Exit;
pub use crate::help::leaf::RefusalContext;
pub use crate::output::declared::Output;

use crate::ToolName;
use crate::help::Help;
use crate::pick::Picker;

#[cfg(test)]
mod tests;

/// What a tool declares to crumbkit, once, as a `const`. It holds the
/// [`Help`], [`Failures`] and [`Output`] records and the picker.
///
/// A tool sets only the records it needs, and each record it leaves out
/// keeps crumbkit's behaviour. It passes the value to
/// [`parse`](crate::parse()), which applies its goal headings to the help
/// of the commands on the typed path, applies the leaf of the deepest
/// command typed, and prints its topics:
///
/// ```
/// use clap::{Parser, Subcommand};
/// use crumbkit::Tool;
/// use crumbkit::help::{Example, Goal, Group, Help, Leaf, Topic};
///
/// /// Browse a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(subcommand)]
///     resource: Resource,
/// }
///
/// #[derive(Subcommand)]
/// enum Resource {
///     /// Work with the books in the catalogue.
///     Books,
/// }
///
/// const TOOL: Tool = Tool::new().help(
///     Help::new()
///         .groups(&[Group::new(
///             &[],
///             &[Goal::new("Browse the catalogue", &["books"])],
///         )])
///         .leaves(&[Leaf::new(&["books"]).examples(&[Example::new(&[], "List the books")])])
///         .topics(&[Topic::new(
///             "catalogue",
///             "Where the books come from",
///             "The catalogue is built into the binary.\n",
///         )]),
/// );
///
/// let args = ["shelf", "books"];
/// let parsed = crumbkit::parse_from::<Cli>(&TOOL, args);
/// assert!(parsed.is_ok());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Tool {
    /// What the tool declares about its help pages.
    help: Help,
    /// What the tool declares about its failures.
    failures: Failures,
    /// What the tool declares about what a run prints.
    output: Output,
    /// The picker that asks a person for a missing argument, or `None` when
    /// `parse` refuses every command line that leaves one out.
    picker: Option<&'static dyn Picker>,
}

impl Tool {
    /// Returns a tool that declares nothing, so it has crumbkit's behaviour
    /// throughout.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            help: Help::new(),
            failures: Failures::new(),
            output: Output::new(),
            picker: None,
        }
    }

    /// Sets what the tool declares about its help pages.
    #[must_use]
    pub const fn help(mut self, help: Help) -> Self {
        self.help = help;
        self
    }

    /// Returns the record that [`Tool::help`] set. The name has clap's
    /// `get_` prefix, because `help` is the builder method.
    #[must_use]
    pub const fn get_help(&self) -> &Help {
        &self.help
    }

    /// Sets what the tool declares about its failures.
    #[must_use]
    pub const fn failures(mut self, failures: Failures) -> Self {
        self.failures = failures;
        self
    }

    /// Returns the record that [`Tool::failures`] set. The name has clap's
    /// `get_` prefix, because `failures` is the builder method.
    #[must_use]
    pub const fn get_failures(&self) -> &Failures {
        &self.failures
    }

    /// Sets what the tool declares about what a run prints.
    #[must_use]
    pub const fn output(mut self, output: Output) -> Self {
        self.output = output;
        self
    }

    /// Returns the record that [`Tool::output`] set. The name has clap's
    /// `get_` prefix, because `output` is the builder method.
    #[must_use]
    pub const fn get_output(&self) -> &Output {
        &self.output
    }

    /// Returns the name that the output of `command`, the tool's clap root,
    /// gives crumbkit's own codes, documents and variables (ADR-0079). It is
    /// the root command's name, or the last word of the path that
    /// [`Help::root`](crate::help::Help::root) sets when that path names a
    /// command, so a cargo subcommand that sets `&["crumbkit"]` is named
    /// `crumbkit`.
    ///
    /// crumbkit-test's checks read the tool's names through it.
    ///
    /// ```
    /// use clap::Command;
    /// use crumbkit::Tool;
    ///
    /// let tool = Tool::new().tool_name(&Command::new("my-tool"));
    /// assert_eq!(tool.schema_id("crumbkit/failure/v1"), "my-tool/failure/v1");
    /// ```
    #[must_use]
    pub fn tool_name(&self, command: &Command) -> ToolName {
        ToolName::new(
            self.help
                .root_in(command)
                .last()
                .copied()
                .unwrap_or_else(|| command.get_name()),
        )
    }

    /// Sets the picker that asks a person for the value of a missing
    /// required argument, such as [`Menu`](crate::pick::Menu) (ADR-0043).
    ///
    /// When clap refuses a command line because it leaves out a required
    /// argument of a leaf, [`parse`](crate::parse) asks the picker to pick the
    /// argument's values from its possible values, or else from the source that
    /// the leaf's [`Leaf::choices`](crate::help::Leaf::choices) names. It asks only when a person can
    /// answer: standard input and standard error are terminals, no coding agent
    /// runs the tool, the caller did not pass `--no-input`, and the tool record
    /// keeps terminal detection on
    /// ([`tool::Output::without_terminal_detection`](crate::tool::Output::without_terminal_detection)).
    /// It then adds the picked values to the command line, prints the whole
    /// command line on standard error after `Running:`, and parses again.
    /// Without a picker, or when it cannot ask, `parse` refuses the command
    /// line as clap does.
    #[must_use]
    pub const fn picker(mut self, picker: &'static dyn Picker) -> Self {
        self.picker = Some(picker);
        self
    }

    /// Returns the picker that [`Tool::picker`] set, or `None`. The name
    /// has clap's `get_` prefix, because `picker` is the builder method.
    #[must_use]
    pub const fn get_picker(&self) -> Option<&'static dyn Picker> {
        self.picker
    }
}

impl Default for Tool {
    /// Returns [`Tool::new`].
    fn default() -> Self {
        Self::new()
    }
}
