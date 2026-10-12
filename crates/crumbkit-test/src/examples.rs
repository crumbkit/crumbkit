//! The example check, which runs every example in a tool's guide against the
//! tool's built binary and pins each run's standard output in a golden file.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};

use clap::Command;
use crumbkit::form::{FailureForm, ReadFailure};
use crumbkit::help::{Guide, Leaf};
use snapbox::Assert;
use snapbox::assert::{Action, DEFAULT_ACTION_ENV};

use crate::allow::{Allow, Allowances, Rule};
use crate::goldens::{compare, remove_stale};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, Xdg};
use crate::tree;

// The tests run `/bin/sh`, which Windows lacks. The two attributes stay
// apart, because clippy reads only a plain `cfg(test)` as test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The settings of [`check_examples`]: the tool's built binary, the
/// directory of the golden files, the environment of each run, the tool's
/// opt-outs and its failure form.
///
/// A tool declares them as a constant in an integration test of the
/// binary's own package, because cargo sets `CARGO_BIN_EXE_<name>`, the
/// path of the built binary, only for that package's integration tests and
/// benchmarks. An integration test can import only from a library, so the
/// package needs a library target that exports the clap command and the
/// guide, and the binary calls that library:
///
/// ```
/// use crumbkit_test::{Allow, Examples, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(Rule::Examples, "books delete", "Each run deletes a book.")];
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples")
///     .env(&[("SHELF_DATA", "tests/fixtures/books.json")])
///     .allow(ALLOW);
/// ```
///
/// cargo runs those tests, and therefore the binary, from the package's
/// manifest directory, so a relative path in `dir` or in an `.env` value
/// resolves from there.
///
/// `Examples` borrows its paths, variables and opt-outs for `'a`. A
/// constant borrows them for `'static`, and a test that builds a path at
/// runtime, such as a temporary golden directory, borrows the `String` that
/// holds it.
#[derive(Clone, Copy, Debug)]
pub struct Examples<'a> {
    /// The path of the tool's built binary.
    binary: &'a str,
    /// The directory of the golden files, which holds only example golden
    /// files.
    dir: &'a str,
    /// The environment variables that each run sets, as name and value.
    env: &'a [(&'a str, &'a str)],
    /// The tool's opt-outs, of which the check reads those from
    /// [`Rule::Examples`].
    allow: &'a [Allow],
    /// The failure form that the tool's guide sets, which reads back the
    /// failure of a run.
    failure_form: Option<&'a dyn FailureForm>,
}

impl<'a> Examples<'a> {
    /// Returns the settings that run the binary at `binary` and keep the
    /// golden files in `dir`, with no environment variables of the tool's
    /// own and no opt-out.
    ///
    /// `dir` holds only example golden files, because the check counts every
    /// `.txt` and `.stderr` file in it that no checked example produces as
    /// stale.
    #[must_use]
    pub const fn new(binary: &'a str, dir: &'a str) -> Self {
        Self {
            binary,
            dir,
            env: &[],
            allow: &[],
            failure_form: None,
        }
    }

    /// Sets the environment variables that each run sets, each a name and a
    /// value, such as `("SHELF_DATA", "tests/fixtures/books.json")`, so a
    /// tool can point its binary at fixture data or a fake server. The XDG
    /// directories that the check sets win over a variable of the same name.
    #[must_use]
    pub const fn env(mut self, env: &'a [(&'a str, &'a str)]) -> Self {
        self.env = env;
        self
    }

    /// Sets the tool's opt-outs. The check skips the examples of each
    /// command that an opt-out from [`Rule::Examples`] names and of every
    /// command below it, and reads no opt-out of another rule, so a tool
    /// passes the same opt-outs that it passes to every other check.
    #[must_use]
    pub const fn allow(mut self, allow: &'a [Allow]) -> Self {
        self.allow = allow;
        self
    }

    /// Sets the failure form that the tool's guide sets with
    /// `Guide::failure_form`, so [`check_twin`](crate::check_twin) reads a
    /// refused run's code through
    /// [`FailureForm::read`](crumbkit::form::FailureForm::read).
    /// [`check_examples`] fails when this form and the guide's form differ.
    #[must_use]
    pub const fn failure_form(mut self, form: &'a dyn FailureForm) -> Self {
        self.failure_form = Some(form);
        self
    }

    /// Returns the path of the tool's built binary.
    pub(crate) const fn binary(&self) -> &'a str {
        self.binary
    }

    /// Returns the environment variables that each run sets.
    pub(crate) const fn variables(&self) -> &'a [(&'a str, &'a str)] {
        self.env
    }

    /// Returns the tool's opt-outs.
    pub(crate) const fn opt_outs(&self) -> &'a [Allow] {
        self.allow
    }

    /// Returns the tool's failure form, or `None`.
    pub(crate) const fn form(&self) -> Option<&'a dyn FailureForm> {
        self.failure_form
    }
}

/// Runs every example in the leaves of `guide` against the binary that
/// `examples` names and checks that each run succeeds and prints the
/// standard output and standard error in its golden files, and panics, as a
/// failed test assertion does, naming every problem it finds.
///
/// A tool calls the check from an integration test of the binary's own
/// package, so that package needs a library target that exports the clap
/// command and the guide (see [`Examples`]).
///
/// The check walks the leaves in declared order and each leaf's examples in
/// order. It runs the binary once for each example, one run at a time, with
/// the leaf's path and the example's words as separate arguments and no
/// shell. It skips a leaf whose path names no command, which
/// [`check_guide`](crate::check_guide) reports, and the leaves of each
/// command that an opt-out of `examples` covers. It runs
/// the examples of a hidden command, because that command's own help lists
/// them as runs that work.
///
/// Each run starts with a cleared environment. The check then sets each
/// variable of [`Examples::env`], and points `XDG_CONFIG_HOME`,
/// `XDG_DATA_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` at fresh
/// directories that it creates for that run and deletes afterwards, so a
/// run never reads or writes the caller's own configuration. The cleared
/// environment has no `PATH` and no `HOME`, so a tool whose binary needs
/// them passes them through [`Examples::env`], such as
/// `("PATH", "/usr/bin:/bin")` or `("HOME", "tests/fixtures/home")`.
///
/// The golden file of an example holds the run's standard output, and
/// [`example_golden_name`] names it after the example's command line, such
/// as `shelf-books-list.txt`, `shelf-books-list-all.txt` and
/// `shelf-books-show-2-5.txt`. A run that writes to standard error, such as
/// a listing's footer and `Next:` line, also has a golden file of its
/// standard error, named with `.stderr` in place of `.txt`, such as
/// `shelf-books-list.stderr`. A run that writes nothing there has no such
/// file (ADR-0005).
///
/// The check compares each output with snapbox, as other golden files are
/// compared (ADR-0004). With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file, rewrites each file whose content differs
/// and deletes each stale file. It finds:
///
/// - a run that exits with a code other than 0, naming the example's command
///   line, the exit status, the failure's code and steps when the check can
///   read them, and the standard error, because an example shows a run that
///   works. The check reads the failure through the form of
///   [`Examples::failure_form`] first, then from a `crumbkit/failure/v1`
///   document on standard output, and then from the last `code:` line on
///   standard error. This holds with `SNAPSHOTS=overwrite` too, and the
///   check then writes no file for the example;
/// - a run that writes a `warning: crumbkit` line to standard error, which a
///   debug build writes for a recovery call that does not render. The check
///   then writes no file for the example;
/// - a guide that sets a failure form with `Guide::failure_form` while
///   [`Examples::failure_form`] names none, or names a form whose document
///   has another id, and an `Examples::failure_form` without a form in the
///   guide;
/// - a run whose binary cannot start, such as a wrong path, naming the path
///   and the error;
/// - a run that does not end within 10 seconds, which the check stops, so a
///   stream command's example reads a finite input and never follows one;
/// - a run of an example whose words hold `--json`, for a leaf whose
///   document has crumbkit's envelope, that prints a line that is not a JSON
///   object whose `schema` is the document's id, or that prints a number of
///   lines other than one when the leaf declares a document of one result.
///   A leaf that declares a stream with `Leaf::stream` may print any number
///   of lines (ADR-0041);
/// - two examples whose golden files have one name, naming both command
///   lines;
/// - an opt-out from [`Rule::Examples`] that names no command or skips no
///   example;
///
/// and, without `SNAPSHOTS=overwrite`:
///
/// - an example without a golden file;
/// - a standard error without a golden file;
/// - an output or a standard error that differs from its golden file, with
///   snapbox's report of both texts;
/// - a stale golden file: a `.txt` file in the directory that no checked
///   example produces, such as the file of a removed or skipped example, or
///   a `.stderr` file that no checked example's standard error produces.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::help::{Example, Guide, Leaf};
/// use crumbkit_test::Examples;
///
/// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
///     .examples(&[Example::new(&["--all"], "Print every book")])]);
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let shelf = Command::new("shelf")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_examples(&shelf, &GUIDE, &EXAMPLES);
/// ```
///
/// # Panics
///
/// Panics when a run, a golden file or an opt-out breaks a rule above, or
/// when the check cannot create a run's XDG directories, or
/// `SNAPSHOTS=overwrite` cannot write or delete a file. The message lists
/// the problems of the command tree and the golden files under the line
/// `the command tree breaks crumbkit's example rules:`, and the runs that
/// fail, including a binary that cannot start, under the line
/// `these runs of the tool's examples fail:`. Each problem starts on a new
/// line, and a problem may continue over several lines: a failed run's
/// problem ends with its standard error, and a differing output's problem
/// ends with snapbox's report. The message names each example by its
/// command line as help prints it, with each word that a POSIX shell would
/// split or expand between single quotes, such as
/// `shelf books list --author 'Jane Austen'`.
pub fn check_examples(command: &Command, guide: &Guide, examples: &Examples<'_>) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, guide, examples, &assert, TIMEOUT);
}

/// Runs [`check_examples`] with `assert`, whose action decides whether the
/// check compares or overwrites, and with `timeout` for each run, so a test
/// chooses the action without setting `SNAPSHOTS` and stops a slow run
/// early.
fn check(
    command: &Command,
    guide: &Guide,
    examples: &Examples<'_>,
    assert: &Assert,
    timeout: Duration,
) {
    let overwrite = assert.selected_action() == Action::Overwrite;
    let dir = Path::new(examples.dir);
    let tool = command.get_name();
    let mut problems = Vec::new();
    // The problems of runs that fail, which the message lists under a
    // heading of their own, because the tool's binary breaks them and the
    // command tree and golden files do not.
    let mut failures = Vec::new();
    let mut allowances = Allowances::new(examples.allow, &[Rule::Examples]);
    let visits = tree::walk(command);
    let form = guide.get_failure_form();
    check_form(form, examples.failure_form, &mut problems);
    // Each golden file name with the command line of the example that has
    // it.
    let mut names: Vec<(String, String)> = Vec::new();
    // The name of each golden file of standard error.
    let mut stderr_names: Vec<String> = Vec::new();
    for leaf in guide.get_leaves() {
        let path = leaf.path();
        let below = path.join(" ");
        if !visits.iter().any(|visit| visit.below() == below) {
            continue;
        }
        if !leaf.get_examples().is_empty() && allowances.excuses(Rule::Examples, &below) {
            continue;
        }
        for example in leaf.get_examples() {
            let args: Vec<&str> = path.iter().chain(example.words()).copied().collect();
            let text = std::iter::once(tool)
                .chain(path.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            let line = command_line(&text, example.words());
            let name = example_golden_name(&line);
            if let Some((_, other)) = names.iter().find(|(known, _)| *known == name) {
                problems.push(format!(
                    "{line}: the example's golden file name `{name}` is also the name for the example `{other}`"
                ));
                continue;
            }
            names.push((name.clone(), line.clone()));
            let output = match run_example(examples, &args, timeout) {
                Ok(output) => output,
                Err(problem) => {
                    failures.push(format!("{line}: {problem}"));
                    continue;
                }
            };
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            if !output.status.success() {
                failures.push(failed_run(&line, &output, form, &stderr));
                continue;
            }
            let warnings = warnings(&line, &stderr);
            if !warnings.is_empty() {
                failures.extend(warnings);
                continue;
            }
            // RUST-10: the check compares the output as UTF-8 text, as the
            // help golden check compares pages. It writes each byte of the
            // output that is not valid UTF-8 as U+FFFD, so it cannot pin
            // binary output. A comparison of `Data::binary` replaces it when
            // a tool's examples print bytes.
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            if example.words().contains(&"--json") {
                check_lines(leaf, &line, &stdout, &mut failures);
            }
            if !stderr.is_empty() {
                let stderr_name = stderr_golden_name(&name);
                compare(
                    assert,
                    &line,
                    "standard error",
                    stderr,
                    &dir.join(&stderr_name),
                    &mut problems,
                );
                stderr_names.push(stderr_name);
            }
            compare(
                assert,
                &line,
                "output",
                stdout,
                &dir.join(&name),
                &mut problems,
            );
        }
    }
    let names: Vec<String> = names.into_iter().map(|(name, _)| name).collect();
    remove_stale(
        dir,
        &names,
        "txt",
        overwrite,
        "checked example",
        &mut problems,
    );
    remove_stale(
        dir,
        &stderr_names,
        "stderr",
        overwrite,
        "checked example that writes to standard error",
        &mut problems,
    );
    allowances.finish(command, &mut problems);
    report(&problems, &failures);
}

/// Runs the example `args` of `examples.binary` in fresh XDG directories and
/// returns its output, or a problem when the run cannot start or does not
/// end within `timeout`.
fn run_example(
    examples: &Examples<'_>,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let started = Instant::now();
    let output = Xdg::new("example").and_then(|xdg| {
        runs::run(
            examples.binary,
            examples.env,
            &xdg,
            args,
            Place::default(),
            timeout,
        )
    })?;
    if !output.status.success() && started.elapsed() >= timeout {
        return Err(format!(
            "the run did not end within {} seconds, so the check stopped it, and an example must end on its own",
            timeout.as_secs_f64()
        ));
    }
    Ok(output)
}

/// Returns the problem of the example `line`, whose run exited with a code
/// other than 0. The problem names the exit status, the failure's code and
/// steps when `form` or crumbkit's failure document reads them, and the
/// run's standard error `stderr` when it is not empty.
fn failed_run(line: &str, output: &Output, form: Option<&dyn FailureForm>, stderr: &str) -> String {
    let mut problem = format!(
        "{line}: an example must exit with code 0, and the run ends with {}",
        output.status
    );
    let failure = runs::read_failure(form, &output.stdout, &output.stderr)
        .or_else(|| runs::text_code(&output.stderr).map(|code| ReadFailure::new(code, Vec::new())));
    if let Some(failure) = failure {
        let _ = write!(problem, " and fails with {}", runs::describe(&failure));
    }
    if !stderr.is_empty() {
        problem.push_str(" after writing this to standard error:\n");
        problem.push_str(stderr);
    }
    problem
}

/// Returns one problem for each line of `stderr` that is a debug build's
/// warning from crumbkit, which marks a mistake in the tool even when the
/// example `line` succeeds.
fn warnings(line: &str, stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|text| text.starts_with("warning: crumbkit "))
        .map(|warning| format!("{line}: the run writes a warning to standard error: {warning}"))
        .collect()
}

/// Adds a problem to `problems` when `set`, the form that the guide sets,
/// and `named`, the form of [`Examples::failure_form`], differ: when only
/// one of them is set, or when their documents have different ids.
fn check_form(
    set: Option<&dyn FailureForm>,
    named: Option<&dyn FailureForm>,
    problems: &mut Vec<String>,
) {
    match (set, named) {
        (Some(set), None) => problems.push(format!(
            "the guide sets the failure form {}, and Examples names no failure form, so check_twin cannot read the tool's failures. Examples::failure_form names the guide's form",
            set.document()
        )),
        (Some(set), Some(named)) if set.document().to_string() != named.document().to_string() => {
            problems.push(format!(
                "the guide sets the failure form {}, and Examples::failure_form names {}",
                set.document(),
                named.document()
            ));
        }
        (None, Some(named)) => problems.push(format!(
            "Examples::failure_form names {}, and the guide sets no failure form",
            named.document()
        )),
        _ => {}
    }
}

/// Returns the name of the golden file of an example's standard error: the
/// name `name` of its output's golden file with `.stderr` in place of
/// `.txt`, such as `shelf-books-list.stderr`.
pub(crate) fn stderr_golden_name(name: &str) -> String {
    format!("{}.stderr", name.strip_suffix(".txt").unwrap_or(name))
}

/// Adds to `failures` each problem of `stdout`, the output of the `--json`
/// example `line` of `leaf`, as [`check_examples`] describes. A leaf
/// without a document, or whose document has no envelope, has none.
fn check_lines(leaf: &Leaf, line: &str, stdout: &str, failures: &mut Vec<String>) {
    let Some(document) = leaf.get_document() else {
        return;
    };
    if !document.has_envelope() {
        return;
    }
    let lines: Vec<&str> = stdout.lines().collect();
    if !leaf.is_stream() && lines.len() != 1 {
        failures.push(format!(
            "{line}: the command prints one document, so its --json output must be exactly one line, and the run printed {} lines",
            lines.len()
        ));
        return;
    }
    let id = document.to_string();
    for (number, text) in (1..).zip(&lines) {
        let schema = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| value.get("schema").cloned());
        if schema.as_ref().and_then(serde_json::Value::as_str) != Some(id.as_str()) {
            failures.push(format!(
                "{line}: line {number} of the --json output must be a JSON object whose schema is {id}"
            ));
        }
    }
}

/// Panics, as a failed test assertion does, when `problems` or `failures`
/// holds any problem. The problems of the command tree and the golden files
/// follow the line `the command tree breaks crumbkit's example rules:`, and
/// the failed runs follow the line `these runs of the tool's examples
/// fail:`, so a reader sees whether the tree or the tool's binary broke.
/// Each problem starts on a new line.
fn report(problems: &[String], failures: &[String]) {
    let mut sections = Vec::new();
    if !problems.is_empty() {
        sections.push(format!(
            "the command tree breaks crumbkit's example rules:\n{}",
            problems.join("\n")
        ));
    }
    if !failures.is_empty() {
        sections.push(format!(
            "these runs of the tool's examples fail:\n{}",
            failures.join("\n")
        ));
    }
    assert!(sections.is_empty(), "{}", sections.join("\n"));
}

/// Returns the name of the golden file that [`check_examples`] keeps for
/// the example whose command line is `line`, as help and `commands list`
/// print it, such as `shelf books list --all`.
///
/// The name is the command line with each run of characters other than
/// ASCII letters, digits, `.` and `_` written as one dash, without a dash at
/// its start or its end, and with `.txt` appended, such as
/// `shelf-books-list-all.txt`. The quotes that `crumbkit::help::quote` adds
/// around a word are such characters, so a quoted word names the file as its
/// unquoted characters do, and `'title, year'` becomes `title-year`.
///
/// A tool that reads its examples from `commands list`, such as
/// `cargo crumbkit weigh`, finds each example's golden file through it.
///
/// ```
/// assert_eq!(
///     crumbkit_test::example_golden_name("shelf books list --all"),
///     "shelf-books-list-all.txt"
/// );
/// ```
#[must_use]
pub fn example_golden_name(line: &str) -> String {
    let stem = line
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_'))
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("{stem}.txt")
}
