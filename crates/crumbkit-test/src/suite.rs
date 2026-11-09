//! The convention suite, which holds the inputs of every check in one record
//! and expands them into one named test for each check.

use std::path::Path;

use clap::Command;
use crumbkit::{Code, Tool};

use crate::allow::{Allow, is_reason};
use crate::{Budgets, Examples, Pages, PathBudgets, Recoveries, Twins};

#[macro_use]
mod check;
#[cfg(test)]
mod tests;

pub use self::check::Check;

/// The inputs of every convention check, which
/// [`conventions!`](crate::conventions) expands into one test for each
/// [`Check`].
///
/// A tool declares one constant in an integration test of its binary's
/// package, because cargo sets `CARGO_BIN_EXE_<name>` only for that package's
/// integration tests:
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::Tool;
/// use crumbkit_test::{Allow, Check, Examples, Rule, Suite};
///
/// const TOOL: Tool = Tool::new();
/// const ALLOW: &[Allow] =
///     &[Allow::new(Rule::RootLeaf, "version", "People type `shelf version` from habit.")];
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// fn command() -> Command {
///     Command::new("shelf")
/// }
///
/// const SUITE: Suite<'_> = Suite::new(command, &TOOL)
///     .allow(ALLOW)
///     .examples(EXAMPLES)
///     .skip(Check::Listings, "shelf has no list commands.");
///
/// crumbkit_test::conventions!(SUITE);
/// # fn main() {}
/// ```
///
/// Each check reads only the inputs that it needs. [`Suite::path_budgets`],
/// [`Suite::examples`], [`Suite::codes`], [`Suite::recoveries`] and
/// [`Suite::twins`] have no default, so a check that needs one fails and names
/// it, unless [`Suite::skip`] gives a written reason. Every other input has a
/// default that the setter's documentation gives.
///
/// The `check_*` functions stay public for a tool that needs one check alone.
#[derive(Clone, Copy, Debug)]
pub struct Suite<'a> {
    /// The tool under test.
    subject: Subject<'a>,
    /// The tool's opt-outs.
    allow: &'a [Allow],
    /// The verbs that the tool adds to crumbkit's own.
    verbs: &'a [&'a str],
    /// The budgets of the tool's help pages.
    budgets: Budgets,
    /// The budgets of the paths through the tool's help.
    path_budgets: Option<PathBudgets<'a>>,
    /// The directory that holds the tool's golden files.
    goldens: &'a str,
    /// The tool's checked-in command reference.
    reference: &'a str,
    /// The tool's binary and the golden files of its examples.
    examples: Option<Examples<'a>>,
    /// Whether the help checks read the pages that the binary prints.
    help_from_binary: bool,
    /// The tool's failure codes.
    codes: Option<&'a [&'a [&'a Code]]>,
    /// The settings of the recovery check.
    recoveries: Option<Recoveries<'a>>,
    /// The settings of the twin check.
    twins: Option<Twins<'a>>,
    /// The reason for each check that the tool skips, indexed by [`Check`].
    skips: [Option<&'a str>; Check::ALL.len()],
}

/// Where a [`Suite`] gets the tool's command tree and tool record.
#[derive(Clone, Copy, Debug)]
enum Subject<'a> {
    /// The tree comes from a function and the record is declared.
    Declared(fn() -> Command, &'a Tool),
    /// A function builds both at run time.
    Built(fn() -> (Command, Tool)),
}

impl<'a> Suite<'a> {
    /// Returns the suite of the tool whose tree `command` returns and whose
    /// tool record is `tool`, with no other input set.
    #[must_use]
    pub const fn new(command: fn() -> Command, tool: &'a Tool) -> Self {
        Self::of(Subject::Declared(command, tool))
    }

    /// Returns the suite of a tool that builds its command tree and tool
    /// record at run time, such as from files it reads at start, with no
    /// other input set.
    #[must_use]
    pub const fn built(build: fn() -> (Command, Tool)) -> Self {
        Self::of(Subject::Built(build))
    }

    /// Returns the suite of `subject` with no other input set.
    const fn of(subject: Subject<'a>) -> Self {
        Self {
            subject,
            allow: &[],
            verbs: &[],
            budgets: Budgets::new(),
            path_budgets: None,
            goldens: "tests/golden",
            reference: "docs/commands.md",
            examples: None,
            help_from_binary: false,
            codes: None,
            recoveries: None,
            twins: None,
            skips: [None; Check::ALL.len()],
        }
    }

    /// Sets the tool's opt-outs, which every check that takes opt-outs reads.
    /// The default is no opt-outs.
    #[must_use]
    pub const fn allow(mut self, allow: &'a [Allow]) -> Self {
        self.allow = allow;
        self
    }

    /// Sets the verbs that [`check_grammar`](crate::check_grammar) accepts
    /// besides crumbkit's own. The default is no other verb.
    #[must_use]
    pub const fn verbs(mut self, verbs: &'a [&'a str]) -> Self {
        self.verbs = verbs;
        self
    }

    /// Sets the budgets of the tool's help pages. The default is
    /// [`Budgets::new`].
    #[must_use]
    pub const fn budgets(mut self, budgets: Budgets) -> Self {
        self.budgets = budgets;
        self
    }

    /// Sets the budgets of the paths through the tool's help, which
    /// [`check_path_weights`](crate::check_path_weights) needs and
    /// [`check_opt_outs`](crate::check_opt_outs) reports.
    #[must_use]
    pub const fn path_budgets(mut self, budgets: PathBudgets<'a>) -> Self {
        self.path_budgets = Some(budgets);
        self
    }

    /// Sets the directory that holds the tool's golden files. The help pages
    /// sit in its `help` directory and the schemas in its `schemas` directory,
    /// and the opt-out report and recovery chains sit in the directory itself.
    /// The default is `tests/golden`.
    #[must_use]
    pub const fn goldens(mut self, dir: &'a str) -> Self {
        self.goldens = dir;
        self
    }

    /// Sets the file of the tool's checked-in command reference. The default
    /// is `docs/commands.md`, where `cargo crumbkit docs` writes it.
    #[must_use]
    pub const fn reference(mut self, file: &'a str) -> Self {
        self.reference = file;
        self
    }

    /// Sets the tool's binary and the settings of its example runs, which
    /// [`check_examples`](crate::check_examples),
    /// [`check_output_budget`](crate::check_output_budget),
    /// [`check_command_reference`](crate::check_command_reference) and
    /// [`Suite::help_from_binary`] need.
    #[must_use]
    pub const fn examples(mut self, examples: Examples<'a>) -> Self {
        self.examples = Some(examples);
        self
    }

    /// Makes the help checks read the pages that the binary of
    /// [`Suite::examples`] prints, as [`Pages::binary`] does. A tool needs it
    /// when it builds part of its help at run time. The default is
    /// [`Pages::rendered`].
    #[must_use]
    pub const fn help_from_binary(mut self) -> Self {
        self.help_from_binary = true;
        self
    }

    /// Sets the tool's failure codes, which
    /// [`check_codes`](crate::check_codes) needs.
    #[must_use]
    pub const fn codes(mut self, codes: &'a [&'a [&'a Code]]) -> Self {
        self.codes = Some(codes);
        self
    }

    /// Sets the settings of [`check_recoveries`](crate::check_recoveries).
    #[must_use]
    pub const fn recoveries(mut self, recoveries: Recoveries<'a>) -> Self {
        self.recoveries = Some(recoveries);
        self
    }

    /// Sets the settings of [`check_twins`](crate::check_twins).
    #[must_use]
    pub const fn twins(mut self, twins: Twins<'a>) -> Self {
        self.twins = Some(twins);
        self
    }

    /// Leaves `check` out of the suite, for `reason`. The test of a skipped
    /// check passes without running it.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A `const`
    /// that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use clap::Command;
    /// use crumbkit::Tool;
    /// use crumbkit_test::{Check, Suite};
    ///
    /// const TOOL: Tool = Tool::new();
    ///
    /// fn command() -> Command {
    ///     Command::new("shelf")
    /// }
    ///
    /// const BAD: Suite<'_> = Suite::new(command, &TOOL).skip(Check::Listings, "");
    /// ```
    #[must_use]
    pub const fn skip(mut self, check: Check, reason: &'a str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        self.skips[check as usize] = Some(reason);
        self
    }

    /// Runs `check`, unless [`Suite::skip`] leaves it out.
    /// [`conventions!`](crate::conventions) calls it once for each check.
    ///
    /// # Panics
    ///
    /// Panics with what the check found, or naming the setter of an input
    /// that the check needs and the suite does not set.
    pub fn run(&self, check: Check) {
        if self.skips[check as usize].is_some() {
            return;
        }
        match self.subject {
            Subject::Declared(command, tool) => self.run_on(check, &command(), tool),
            Subject::Built(build) => {
                let (command, tool) = build();
                self.run_on(check, &command, &tool);
            }
        }
    }

    /// Runs `check` on the tool's tree and tool record.
    fn run_on(&self, check: Check, command: &Command, tool: &Tool) {
        let pages = || self.pages(check, tool);
        let examples = || required(self.examples.as_ref(), check, "examples");
        let goldens = Path::new(self.goldens);
        match check {
            Check::Names => crate::check_names(command, pages(), self.allow),
            Check::Grammar => crate::check_grammar(command, pages(), self.verbs, self.allow),
            Check::Effects => crate::check_effects(command, tool, self.allow),
            Check::Listings => crate::check_listings(command, self.allow),
            Check::Filters => crate::check_filters(command, self.allow),
            Check::Help => crate::check_help(command, tool, self.allow),
            Check::ExitCodes => crate::check_exit_codes(command, tool, self.allow),
            Check::Codes => {
                crate::check_codes(tool, required(self.codes, check, "codes"));
            }
            Check::HelpSizes => crate::check_help_sizes(command, pages(), &self.budgets),
            Check::PathWeights => crate::check_path_weights(
                command,
                pages(),
                required(self.path_budgets.as_ref(), check, "path_budgets"),
            ),
            Check::HelpGoldens => {
                crate::check_help_goldens(command, pages(), goldens.join("help"));
            }
            Check::SchemaGoldens => {
                crate::check_schema_goldens(command, tool, goldens.join("schemas"));
            }
            Check::DocumentIds => crate::check_document_ids(command, tool, self.allow),
            Check::Mounts => crate::check_mounts(command, tool),
            Check::Examples => crate::check_examples(command, tool, examples()),
            Check::OutputBudget => crate::check_output_budget(command, tool, examples()),
            Check::CommandReference => {
                crate::check_command_reference(examples().binary(), self.reference);
            }
            Check::OptOuts => crate::check_opt_outs(
                command,
                tool,
                self.allow,
                &self.budgets,
                self.path_budgets.as_ref(),
                goldens,
            ),
            Check::Recoveries => crate::check_recoveries(
                command,
                tool,
                required(self.recoveries.as_ref(), check, "recoveries"),
                self.goldens,
            ),
            Check::Twins => {
                crate::check_twins(command, tool, required(self.twins.as_ref(), check, "twins"));
            }
        }
    }

    /// Returns the help pages that `check` reads.
    fn pages<'s>(&'s self, check: Check, tool: &'s Tool) -> Pages<'s> {
        if self.help_from_binary {
            Pages::binary(tool, required(self.examples.as_ref(), check, "examples"))
        } else {
            Pages::rendered(tool)
        }
    }
}

/// Returns `input`, or panics naming its setter and the check that needs it.
fn required<'s, T: ?Sized>(input: Option<&'s T>, check: Check, setter: &str) -> &'s T {
    let Some(input) = input else {
        #[expect(
            clippy::panic,
            reason = "a check reports a broken rule by panicking, as a failed test assertion does"
        )]
        {
            panic!(
                "the check {check:?} needs an input that this suite does not set: call \
                 .{setter}(...) on the Suite, or skip the check with \
                 .skip(Check::{check:?}, \"the reason\")"
            )
        }
    };
    input
}
