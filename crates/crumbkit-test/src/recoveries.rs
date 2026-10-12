//! The recovery check, which runs command lines that fail against a tool's
//! built binary and follows each `run` recovery that they print.

use std::fmt::Write as _;
use std::path::Path;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};
use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::allow::{Allowances, Rule, is_reason};
use crate::examples::Examples;
use crate::goldens::compare;
use crate::runs::Dir;

mod chain;
// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// One command line that fails, which [`check_recoveries`] runs to start a
/// chain of recoveries: the code it fails with, its words after the tool's
/// name, and the working directory and standard input of its runs.
///
/// ```
/// use crumbkit::Code;
/// use crumbkit_test::Failing;
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
///
/// const SHOW_7: Failing<'_> = Failing::new(&NOT_FOUND, &["books", "show", "7"]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Failing<'a> {
    /// The code that the command line fails with.
    code: &'a Code,
    /// The words after the tool's name.
    words: &'a [&'a str],
    /// The working directory of every run of the chain.
    dir: Option<&'a str>,
    /// The standard input of every run of the chain.
    stdin: Option<&'a str>,
    /// The XDG fixture of the chain, which wins over the fixture of
    /// [`Examples::xdg`].
    xdg: Option<&'a str>,
    /// The value of each placeholder in the steps of the chain, which wins
    /// over every other source.
    values: &'a [(&'a str, &'a str)],
}

impl<'a> Failing<'a> {
    /// Returns the command line of `words`, the words after the tool's name,
    /// which fails with `code`. Its runs start in the directory of
    /// [`Examples::current_dir`], or in the test's directory, and read an
    /// empty standard input.
    #[must_use]
    pub const fn new(code: &'a Code, words: &'a [&'a str]) -> Self {
        Self {
            code,
            words,
            dir: None,
            stdin: None,
            xdg: None,
            values: &[],
        }
    }

    /// Sets the working directory of the command line and of every step of
    /// its chain, relative to the test's directory or absolute. Without it,
    /// the runs start in the directory of [`Examples::current_dir`], in one
    /// copy of the fixture of [`Examples::fresh_dir`] for the whole chain,
    /// or in the test's directory when neither is set.
    #[must_use]
    pub const fn dir(mut self, dir: &'a str) -> Self {
        self.dir = Some(dir);
        self
    }

    /// Sets the text on standard input of the command line and of every
    /// step of its chain.
    #[must_use]
    pub const fn stdin(mut self, stdin: &'a str) -> Self {
        self.stdin = Some(stdin);
        self
    }

    /// Sets the XDG fixture of the command line and of every step of its
    /// chain, which wins over the fixture of [`Examples::xdg`] and follows
    /// the same rules. [`check_twin`](crate::check_twin) runs both of its
    /// lines in a copy of the refused line's fixture.
    #[must_use]
    pub const fn xdg(mut self, dir: &'a str) -> Self {
        self.xdg = Some(dir);
        self
    }

    /// Sets the value of each placeholder in the steps of its chain, as a
    /// name and a value, such as `("path", "/tasks/a")`, so each chain can
    /// fill a placeholder with the value that fits its own failure. A value
    /// matches a placeholder's name exactly, case included, as
    /// [`Recoveries::values`] does, and wins over a field of the failure
    /// and over [`Recoveries::values`].
    #[must_use]
    pub const fn values(mut self, values: &'a [(&'a str, &'a str)]) -> Self {
        self.values = values;
        self
    }

    /// Returns the value of each placeholder in the steps of its chain.
    pub(crate) const fn own_values(&self) -> &'a [(&'a str, &'a str)] {
        self.values
    }

    /// Returns the XDG fixture of its runs: its own, or `fallback`, the
    /// fixture of [`Examples::xdg`].
    pub(crate) fn fixture<'p>(&self, fallback: Option<&'p str>) -> Option<&'p str>
    where
        'a: 'p,
    {
        self.xdg.or(fallback)
    }

    /// Returns the code that the command line fails with.
    pub(crate) const fn code(&self) -> &'a Code {
        self.code
    }

    /// Returns the words after the tool's name.
    pub(crate) const fn words(&self) -> &'a [&'a str] {
        self.words
    }

    /// Returns the working directory of its runs. A command line without a
    /// directory of its own runs in `fallback`, the directory or fresh
    /// fixture of [`Examples`], or in the test's directory when that is
    /// `None` too.
    pub(crate) fn work_dir<'p>(&self, fallback: Option<Dir<'p>>) -> Option<Dir<'p>>
    where
        'a: 'p,
    {
        self.dir.map(Dir::Fixed).or(fallback)
    }

    /// Returns the bytes on standard input of its runs, or `None`.
    pub(crate) fn stdin_bytes(&self) -> Option<&'a [u8]> {
        self.stdin.map(str::as_bytes)
    }
}

/// A declared code that no command line in a test can provoke, with the
/// reason, such as a failure that needs a full disk.
#[derive(Clone, Copy, Debug)]
pub struct Unprovoked<'a> {
    /// The declared code.
    code: &'a Code,
    /// Why no test can provoke it, in one line.
    reason: &'a str,
}

impl<'a> Unprovoked<'a> {
    /// Returns the entry that lists `code` as unprovoked for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::Code;
    /// use crumbkit_test::Unprovoked;
    ///
    /// const GONE: Code = Code::new("shelf", "books", "removed_file");
    /// const BAD: Unprovoked<'_> = Unprovoked::new(&GONE, "");
    /// ```
    #[must_use]
    pub const fn new(code: &'a Code, reason: &'a str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self { code, reason }
    }
}

/// The settings of [`check_recoveries`]: how to run the tool's binary, how
/// the tool spells its codes, its declared codes, the command lines that
/// start the chains, the declared codes that no command line provokes, the
/// values of placeholders and the most steps of a chain.
///
/// ```
/// use crumbkit::{Code, Spelling};
/// use crumbkit_test::{Examples, Failing, Recoveries, Unprovoked};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// const GONE: Code = Code::new("shelf", "books", "removed_file");
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// const RECOVERIES: Recoveries<'_> =
///     Recoveries::new(EXAMPLES, Spelling::COLONS, &[&[&NOT_FOUND, &GONE]])
///         .starts(&[Failing::new(&NOT_FOUND, &["books", "show", "7"])])
///         .unprovoked(&[Unprovoked::new(&GONE, "Only a book file that disappears provokes it.")])
///         .values(&[("ID", "2")]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Recoveries<'a> {
    /// The binary, its environment and the tool's opt-outs.
    examples: Examples<'a>,
    /// How the tool spells its codes.
    spelling: Spelling,
    /// The `CODES` list of each of the tool's error types.
    codes: &'a [&'a [&'a Code]],
    /// The command lines that start the chains.
    starts: &'a [Failing<'a>],
    /// The declared codes that no command line provokes.
    unprovoked: &'a [Unprovoked<'a>],
    /// The value of each placeholder, by the name between its angle
    /// brackets.
    values: &'a [(&'a str, &'a str)],
    /// The most steps that one chain runs after its start.
    steps: u8,
}

impl<'a> Recoveries<'a> {
    /// Returns the settings that run the binary of `examples` with its
    /// environment and opt-outs, spell codes with `spelling` and check the
    /// codes of `codes`, the `CODES` list of each of the tool's error types,
    /// with no start, no unprovoked code, no value and at most 3 steps in a
    /// chain.
    #[must_use]
    pub const fn new(
        examples: Examples<'a>,
        spelling: Spelling,
        codes: &'a [&'a [&'a Code]],
    ) -> Self {
        Self {
            examples,
            spelling,
            codes,
            starts: &[],
            unprovoked: &[],
            values: &[],
            steps: 3,
        }
    }

    /// Sets the command lines that start the chains.
    #[must_use]
    pub const fn starts(mut self, starts: &'a [Failing<'a>]) -> Self {
        self.starts = starts;
        self
    }

    /// Sets the declared codes that no command line in a test provokes.
    #[must_use]
    pub const fn unprovoked(mut self, unprovoked: &'a [Unprovoked<'a>]) -> Self {
        self.unprovoked = unprovoked;
        self
    }

    /// Sets the value of each placeholder, as a name and a value, such as
    /// `("ID", "2")` for the placeholder `<ID>...` in the recovery
    /// `shelf books show <ID>...`. A placeholder's name may hold capital or
    /// lowercase letters, and a value fills only the placeholder whose name
    /// matches it exactly, case included, so `("path", "x")` fills `<path>`
    /// and not `<PATH>`.
    ///
    /// Every chain shares these values. A value of
    /// [`Failing::values`] wins over them for one chain, and so does a field
    /// of the failure that offers the step, as [`check_recoveries`]
    /// describes.
    #[must_use]
    pub const fn values(mut self, values: &'a [(&'a str, &'a str)]) -> Self {
        self.values = values;
        self
    }

    /// Sets the most steps that one chain runs after its start.
    #[must_use]
    pub const fn steps(mut self, steps: u8) -> Self {
        self.steps = steps;
        self
    }
}

/// Runs each command line that `recoveries` starts with against the tool's
/// binary, follows every `run` recovery that the failures print, writes the
/// chains in the golden file `recoveries.txt` in `dir`, and panics, as a
/// failed test assertion does, naming every problem it finds.
///
/// A tool calls the check from an integration test of the binary's own
/// package, as it calls [`check_examples`](crate::check_examples).
///
/// Every run adds `--json` when its words lack it. When the guide sets a
/// failure form, the check reads each failed run through the form's
/// [`FailureForm::read`](crumbkit::form::FailureForm::read) first.
/// Otherwise, or when `read` returns `None`, it reads the failure of the
/// last line of standard output that is a `crumbkit/failure/v1` document or
/// that has `failures`, as a batch prints them. It follows the first entry
/// of a batch's `failures`. The runs of
/// one chain share one set of fresh XDG directories, which start as copies
/// of the fixture of [`Failing::xdg`] or [`Examples::xdg`], the working directory
/// and the standard input of its start, so a step sees what the earlier
/// runs wrote. A chain whose working directory comes from
/// [`Examples::fresh_dir`] runs in one new copy of that fixture.
///
/// A chain follows the recovery and then each alternative of a failure. A
/// `run` step fills each placeholder, such as `<ID>...`, drops the tool's
/// name and runs the words. A placeholder takes the first value that one of
/// these sources gives:
///
/// 1. the value of [`Failing::values`] whose name matches it exactly;
/// 2. the field, in the `fields` of crumbkit's failure document or of a
///    batch's first failure, of the failure that offers the step, whose name
///    matches it with capitals lowered and `-` read as `_`, so the field
///    `book_id` fills `<BOOK-ID>`. An integer or a boolean is written as
///    text. A failure that only the tool's own form reads gives no fields;
/// 3. the value of [`Recoveries::values`] whose name matches it exactly.
///
/// A step that exits with code 0 ends its branch, and so do a `retry`, a `hint` and
/// a `none` step. A step that fails continues the chain, up to
/// [`Recoveries::steps`] steps after the start. A chain does not run a
/// command line that it ran already, a step that an opt-out from
/// [`Rule::Recoveries`] covers, or the recovery of a code that the tool
/// does not declare and crumbkit does not own, such as a run-time code from
/// a plugin. The check counts such codes in the report.
///
/// It finds:
///
/// - a start whose code is not declared, which it does not run;
/// - a start that does not fail with its code;
/// - a step that the parser refuses with crumbkit's usage code;
/// - a step that fails with a code that no declared code spells, when
///   [`Spelling::split`](crumbkit::Spelling::split) reads the code's
///   component as the component of a declared code, so a code missing from
///   a `CODES` list is found;
/// - a step that fails again with the code of the failure it follows,
///   unless an opt-out from [`Rule::SameCode`] covers the step's command;
/// - a failed run whose failure the tool's form, set with
///   `Guide::failure_form`, does not read through
///   [`FailureForm::read`](crumbkit::form::FailureForm::read), and that
///   prints no `crumbkit/failure/v1` document. The first such run stops the
///   check, which reports this problem alone and says to implement `read`
///   or to add an opt-out from [`Rule::Recoveries`] that names the root
///   (ADR-0045);
/// - a run that writes a `warning: crumbkit` line to standard error, which a
///   debug build writes for a recovery call that does not render;
/// - a step that crumbkit's quoting cannot read, that does not start with
///   the tool's name, or that has a placeholder without a value;
/// - a run that cannot start, that does not end within 10 seconds or that
///   fails without a failure document;
/// - a declared code that no chain provokes and [`Recoveries::unprovoked`]
///   does not list, and a listed code that is not declared or that a chain
///   provokes;
/// - a golden file that differs, or is missing without
///   `SNAPSHOTS=overwrite`;
/// - an opt-out from [`Rule::Recoveries`] or [`Rule::SameCode`] that names
///   no command or excuses no problem.
///
/// An opt-out from [`Rule::Recoveries`] skips each start below its command.
///
/// # Panics
///
/// Panics when a rule above is broken. The message lists each problem on
/// its own line under `the tool's recoveries break crumbkit's recovery
/// rules:`.
pub fn check_recoveries(command: &Command, guide: &Guide, recoveries: &Recoveries<'_>, dir: &str) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, guide, recoveries, dir, &assert);
}

/// Runs [`check_recoveries`] with `assert`, whose action decides whether
/// the check compares or overwrites the golden file.
fn check(
    command: &Command,
    guide: &Guide,
    recoveries: &Recoveries<'_>,
    dir: &str,
    assert: &Assert,
) {
    let spelling = recoveries.spelling;
    let mut declared: Vec<(String, &Code)> = Vec::new();
    for &code in recoveries.codes.iter().flat_map(|list| list.iter()) {
        if !declared.iter().any(|&(_, seen)| seen == code) {
            declared.push((spelling.spell(code), code));
        }
    }
    let mut known: Vec<String> = declared.iter().map(|(text, _)| text.clone()).collect();
    known.extend(
        crumbkit::codes::ALL
            .iter()
            .map(|&code| spelling.spell(code)),
    );
    // crumbkit's add-on crates, such as crumbkit-aliases, own codes in the
    // `crumbkit` component that `crumbkit::codes::ALL` does not list, so
    // only the declared codes give components.
    let mut components: Vec<String> = Vec::new();
    for (text, _) in &declared {
        if let Some((component, _, _)) = spelling.split(text)
            && !components.iter().any(|seen| seen == component)
        {
            components.push(component.to_owned());
        }
    }
    let rules = [Rule::Recoveries, Rule::SameCode];
    let allowances = Allowances::new(recoveries.examples.opt_outs(), &rules);
    let mut walk = chain::Walk {
        recoveries,
        command,
        known,
        components,
        usage: spelling.spell(&crumbkit::codes::CLI_USAGE),
        allowances,
        problems: Vec::new(),
        report: String::new(),
        provoked: Vec::new(),
        skipped: 0,
        form: guide.get_failure_form(),
        stopped: None,
    };
    for start in recoveries.starts {
        walk.start(start);
    }
    if let Some(problem) = walk.stopped {
        fail(&[problem]);
        return;
    }
    let chain::Walk {
        allowances,
        mut problems,
        mut report,
        provoked,
        skipped,
        ..
    } = walk;
    for (text, _) in &declared {
        let listed = recoveries
            .unprovoked
            .iter()
            .any(|entry| spelling.spell(entry.code) == *text);
        if !listed && !provoked.contains(text) {
            problems.push(format!(
                "{text}: no start provokes the declared code, and Recoveries::unprovoked does not list it"
            ));
        }
    }
    report.push_str("codes that no start provokes:\n");
    for entry in recoveries.unprovoked {
        let text = spelling.spell(entry.code);
        if !declared.iter().any(|&(_, code)| code == entry.code) {
            problems.push(format!(
                "{text}: Recoveries::unprovoked lists the code, and it is not a declared code"
            ));
        } else if provoked.contains(&text) {
            problems.push(format!(
                "{text}: Recoveries::unprovoked lists the code, and a start provokes it"
            ));
        }
        let _ = writeln!(report, "  {text}: {}", entry.reason);
    }
    let _ = writeln!(
        report,
        "\ncodes whose recoveries the check did not follow, because no declared code spells them: {skipped}"
    );
    let file = Path::new(dir).join("recoveries.txt");
    let line = format!("{} recoveries", command.get_name());
    compare(assert, &line, "report", report, &file, &mut problems);
    allowances.finish(command, &mut problems);
    fail(&problems);
}

/// Panics, as a failed test assertion does, when `problems` holds a
/// problem, with each problem on its own line.
fn fail(problems: &[String]) {
    assert!(
        problems.is_empty(),
        "the tool's recoveries break crumbkit's recovery rules:\n{}",
        problems.join("\n")
    );
}
