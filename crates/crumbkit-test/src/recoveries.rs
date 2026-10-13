//! The recovery check, which runs command lines that fail against a tool's
//! built binary and follows each `run` recovery that they print.

use std::fmt::Write as _;
use std::path::Path;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};
use snapbox::Assert;
use snapbox::assert::DEFAULT_ACTION_ENV;

use crate::allow::{Allowances, Rule};
use crate::codes::Declared;
use crate::examples::Examples;
use crate::failures;
use crate::goldens::compare;
use crate::runs::{Dir, Location};

mod chain;
mod runtime;
mod unprovoked;
mod words;

pub use runtime::RuntimeFailing;
pub use unprovoked::Unprovoked;

// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// One command line that fails, which [`check_recoveries`] runs to start a
/// chain of recoveries: the code it fails with, its words after the tool's
/// name, and the working directory, standard input and environment
/// variables of its runs.
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
    dir: Option<Location<'a>>,
    /// The standard input of every run of the chain.
    stdin: Option<&'a str>,
    /// The XDG fixture of the chain, which wins over the fixture of
    /// [`Examples::xdg`].
    xdg: Option<Location<'a>>,
    /// The value of each placeholder in the steps of the chain, which wins
    /// over every other source.
    values: &'a [(&'a str, &'a str)],
    /// The variables that every run of the chain sets or removes.
    env: &'a [(&'a str, Option<&'a str>)],
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
            env: &[],
        }
    }

    /// Sets the working directory of the command line and of every step of
    /// its chain, relative to the test's directory or absolute. Without it,
    /// the runs start in the directory of [`Examples::current_dir`], in one
    /// copy of the fixture of [`Examples::fresh_dir`] for the whole chain,
    /// or in the test's directory when neither is set.
    #[must_use]
    pub const fn dir(mut self, dir: &'a str) -> Self {
        self.dir = Some(Location::Text(dir));
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
        self.xdg = Some(Location::Text(dir));
        self
    }

    /// Sets the working directory to `dir`, a path that need not be UTF-8,
    /// as [`Failing::dir`] sets it.
    pub(crate) const fn dir_path(mut self, dir: &'a Path) -> Self {
        self.dir = Some(Location::Path(dir));
        self
    }

    /// Sets the XDG fixture to `dir`, a path that need not be UTF-8, as
    /// [`Failing::xdg`] sets it.
    pub(crate) const fn xdg_path(mut self, dir: &'a Path) -> Self {
        self.xdg = Some(Location::Path(dir));
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

    /// Sets environment variables of the command line and of every step of
    /// its chain, as a name and a value, such as `("DEPTH", Some("1"))`, or
    /// `None` to remove the variable, such as `("HOME", None)`. Each entry
    /// wins over [`Examples::env`] and over the fresh XDG directories of the
    /// check. A problem and the report name the line with its variables,
    /// such as `tool books list (with DEPTH=1)`.
    /// [`Twin::variable`](crate::Twin::variable) makes a twin that differs
    /// by one of them.
    #[must_use]
    pub const fn env(mut self, env: &'a [(&'a str, Option<&'a str>)]) -> Self {
        self.env = env;
        self
    }

    /// Returns the variables that its runs set or remove.
    pub(crate) const fn own_env(&self) -> &'a [(&'a str, Option<&'a str>)] {
        self.env
    }

    /// Returns the value of each placeholder in the steps of its chain.
    pub(crate) const fn own_values(&self) -> &'a [(&'a str, &'a str)] {
        self.values
    }

    /// Returns the XDG fixture of its runs: its own, or `fallback`, the
    /// fixture of [`Examples::xdg`].
    pub(crate) fn fixture<'p>(&self, fallback: Option<&'p Path>) -> Option<&'p Path>
    where
        'a: 'p,
    {
        self.xdg.map(Location::path).or(fallback)
    }

    /// Returns the same command line with the words `words` in place of
    /// its own, with its directory, input, fixture, values and variables.
    pub(crate) const fn with_words<'b>(&self, words: &'b [&'b str]) -> Failing<'b>
    where
        'a: 'b,
    {
        Failing {
            code: self.code,
            words,
            dir: self.dir,
            stdin: self.stdin,
            xdg: self.xdg,
            values: self.values,
            env: self.env,
        }
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
    /// The tool's declared codes.
    codes: Declared<'a>,
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
            codes: Declared::Lists(codes),
            starts: &[],
            unprovoked: &[],
            values: &[],
            steps: 3,
        }
    }

    /// Returns the settings that [`Recoveries::new`] returns, for `codes`,
    /// one list of the tool's declared codes held by value, such as codes
    /// that the tool builds at run time.
    #[must_use]
    pub const fn from_codes(examples: Examples<'a>, spelling: Spelling, codes: &'a [Code]) -> Self {
        Self {
            codes: Declared::Codes(codes),
            ..Self::new(examples, spelling, &[])
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
/// Every run adds `--json` when its words lack it, before a `--` word or at
/// the end, so the tool reads it as a flag. When the guide sets a
/// failure form, the check reads each failed run through the form's
/// [`FailureForm::read`](crumbkit::form::FailureForm::read) first, and when
/// [`Examples`] sets a [`FailureReader`](crate::FailureReader), through
/// that reader first. Otherwise, or when neither reads a failure, it reads
/// the failure of the last document on standard output that is a
/// `crumbkit/failure/v1` document or that has `failures`, as a batch prints
/// them, and then the last `code:` line on standard error, which gives a
/// failure without steps or fields. It follows every failure that it
/// reads, such as each entry of a batch's `failures`, and each failure
/// starts its own branch, which may run a command line that a sibling
/// branch ran. A start passes when any of its failures carries the start's
/// code. The runs of
/// one chain share one set of fresh XDG directories, which start as copies
/// of the fixture of [`Failing::xdg`] or [`Examples::xdg`], the working directory
/// and the standard input of its start, so a step sees what the earlier
/// runs wrote. A chain whose working directory comes from
/// [`Examples::fresh_dir`] runs in one new copy of that fixture.
///
/// A chain follows the recovery and then each alternative of a failure. A
/// `run` step fills each placeholder, such as `<ID>...` or each of the two
/// in `<plugin>.<capability>`, keeps the text between placeholders in a
/// word, drops the tool's name and runs the words. A placeholder takes the first value that one of
/// these sources gives:
///
/// 1. the value of [`Failing::values`] whose name matches it exactly;
/// 2. the field of the failure that offers the step whose name matches it
///    with capitals lowered and `-` read as `_`, so the field `book_id`
///    fills `<BOOK-ID>`. The fields are those that the form or reader
///    returns with the failure, or the `fields` of crumbkit's failure
///    document. A text field fills as it is, an integer or a boolean as
///    JSON writes it, and a null, list or object field fills no
///    placeholder;
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
/// - a start that exits with code 0, or none of whose failures carries its
///   code;
/// - a step that the parser refuses with crumbkit's usage code;
/// - a step that fails with a code that no declared code spells, when
///   [`Spelling::split`](crumbkit::Spelling::split) reads the code's
///   component as the component of a declared code, so a code missing from
///   a `CODES` list is found;
/// - a step that fails with the code of the failure it follows or of an
///   earlier failure of its chain, unless an opt-out from
///   [`Rule::SameCode`] covers the step's command;
/// - a failure with one of the tool's declared codes that offers no `run`,
///   `retry` or `hint` step, such as a failure whose recovery is
///   `Recovery::None` with a reason, unless an opt-out from
///   [`Rule::NextStep`] covers the command that failed. A failure from a
///   run that writes a `warning: crumbkit` line is not also reported here;
/// - a placeholder that a field of the failure fills, which the report
///   also shows under the step, unless an opt-out from
///   [`Rule::FieldValue`] covers the command whose failure printed the
///   step. A recovery built with `Call::value` carries the value, so an
///   agent need not copy it from the field. A value of [`Failing::values`]
///   that wins over the field is no finding;
/// - a guide that sets a failure form and examples that set a failure
///   reader with [`Examples::failure_reader`], which stops the check before
///   any run;
/// - a failed run whose failure neither the tool's form, set with
///   `Guide::failure_form`, nor its reader, set with
///   [`Examples::failure_reader`], reads, and that prints no
///   `crumbkit/failure/v1` document and no `code:` line. The first such run
///   stops the check, which reports this problem alone and says to make
///   the form or reader read it or to add an opt-out from
///   [`Rule::Recoveries`] that names the root (ADR-0045);
/// - a run that writes a `warning: crumbkit` line to standard error, which a
///   debug build writes for a recovery call that does not render and for a
///   run whose `main` never called `Output::set_command`;
/// - a step that crumbkit's quoting cannot read, that does not start with
///   the tool's name, or that has a placeholder without a value;
/// - a run that cannot start, that does not end within 10 seconds or that
///   fails without a failure that the check can read;
/// - a failed run whose exit code breaks the guide's exits, as
///   [`check_twin`](crate::check_twin) finds for a refused line;
/// - a declared code that no chain provokes and [`Recoveries::unprovoked`]
///   does not list, and a listed code that is not declared or that a chain
///   provokes;
/// - a golden file that differs, or is missing without
///   `SNAPSHOTS=overwrite`;
/// - an opt-out from [`Rule::Recoveries`], [`Rule::SameCode`],
///   [`Rule::FieldValue`] or [`Rule::NextStep`] that names no command or
///   excuses no problem.
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
    let reader = match failures::reader(guide, &recoveries.examples) {
        Ok(reader) => reader,
        Err(problem) => {
            fail(&[problem]);
            return;
        }
    };
    let spelling = recoveries.spelling;
    let mut declared: Vec<(String, &Code)> = Vec::new();
    for code in recoveries.codes.to_vec() {
        if !declared.iter().any(|&(_, seen)| seen == code) {
            declared.push((spelling.spell(code), code));
        }
    }
    let mut known: Vec<String> = declared.iter().map(|(text, _)| text.clone()).collect();
    known.extend(
        crumbkit::codes::ALL
            .iter()
            .map(|&code| spelling.spell(guide.printed(code))),
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
    let rules = [
        Rule::Recoveries,
        Rule::SameCode,
        Rule::FieldValue,
        Rule::NextStep,
    ];
    let allowances = Allowances::new(recoveries.examples.opt_outs(), &rules);
    let mut walk = chain::Walk {
        recoveries,
        command,
        declared: declared.iter().map(|(text, _)| text.clone()).collect(),
        known,
        components,
        usage: spelling.spell(guide.printed(&crumbkit::codes::CLI_USAGE)),
        allowances,
        problems: Vec::new(),
        report: String::new(),
        provoked: Vec::new(),
        skipped: 0,
        reader,
        guide,
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
            .any(|entry| spelling.spell(entry.code()) == *text);
        if !listed && !provoked.contains(text) {
            problems.push(format!(
                "{text}: no start provokes the declared code, and Recoveries::unprovoked does not list it"
            ));
        }
    }
    report.push_str("codes that no start provokes:\n");
    for entry in recoveries.unprovoked {
        let text = spelling.spell(entry.code());
        if !declared.iter().any(|&(_, code)| code == entry.code()) {
            problems.push(format!(
                "{text}: Recoveries::unprovoked lists the code, and it is not a declared code"
            ));
        } else if provoked.contains(&text) {
            problems.push(format!(
                "{text}: Recoveries::unprovoked lists the code, and a start provokes it"
            ));
        }
        let _ = writeln!(report, "  {text}: {}", entry.reason());
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
