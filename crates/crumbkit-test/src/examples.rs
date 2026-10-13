//! The example check, which runs every example in a tool's guide against the
//! tool's built binary and pins each run's standard output in a golden file.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Output;
use std::time::{Duration, Instant};

use clap::Command;
use crumbkit::form::{FailureForm, ReadFailure};
use crumbkit::help::Guide;
use snapbox::assert::{Action, DEFAULT_ACTION_ENV};
use snapbox::{Assert, Redactions};

use crate::allow::{Allowances, Rule};
use crate::goldens::{compare, remove_stale};
use crate::guide::command_line;
use crate::runs::{self, Place, TIMEOUT, WorkDir, Xdg};
use crate::tree;

pub(crate) use self::inputs::read_stdin;
pub use self::settings::Examples;
pub(crate) use self::settings::setup_problem;

mod inputs;
mod lines;
mod settings;
// The tests run `/bin/sh`, which Windows lacks. The two attributes stay
// apart, because clippy reads only a plain `cfg(test)` as test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

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
/// [`check_guide`](crate::check_guide) reports, the leaves of each
/// command that an opt-out of `examples` covers, and each example that an
/// [`Allow::example`](crate::Allow::example) names. It runs
/// the examples of a hidden command, because that command's own help lists
/// them as runs that work.
///
/// Each run starts with a cleared environment. The check then sets each
/// variable of [`Examples::env`], and points `XDG_CONFIG_HOME`,
/// `XDG_DATA_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME` at fresh
/// directories that it creates for that run and deletes afterwards, so a
/// run never reads or writes the caller's own configuration. The fresh
/// directories start as copies of the fixture of [`Examples::xdg`] when
/// the tool sets one. The cleared
/// environment has no `PATH` and no `HOME`, so a tool whose binary needs
/// them passes them through [`Examples::env`], such as
/// `("PATH", "/usr/bin:/bin")` or `("HOME", "tests/fixtures/home")`.
///
/// Each run starts in the directory of [`Examples::current_dir`], or in a
/// new copy of the fixture of [`Examples::fresh_dir`], or in the test's
/// directory without either. The check fails without running any example
/// when that directory or fixture does not exist. Each run reads the bytes of
/// the example's input file on standard input, a file in the golden
/// directory named with `.stdin` in place of `.txt`, such as
/// `shelf-books-import.stdin`. A run without that file reads an empty
/// standard input. The check writes the whole input before the 10-second
/// limit starts, so an input file holds at most 64 KiB: a tool that does
/// not read a larger input blocks the check, because the operating
/// system's pipe holds about 64 KiB.
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
/// The check compares each output and standard error with its golden file
/// byte for byte through snapbox (ADR-0004), after it replaces each value of
/// [`Examples::redact`] and the path of each of the run's XDG directories,
/// such as the directory of `XDG_CONFIG_HOME`, with its placeholder, such
/// as `[XDG_CONFIG_HOME]`, and the path of the copy of
/// [`Examples::fresh_dir`] with `[ROOT]`. snapbox's wildcards, such as `[..]`, are plain
/// text in an example golden file. With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file, rewrites each file whose content differs
/// and deletes each stale file. It finds:
///
/// - a run that exits with a code other than 0, naming the example's command
///   line, the exit status, the failure's code and steps when the check can
///   read them, and the standard error, because an example shows a run that
///   works. The check reads the failure through the guide's failure form
///   first, then from a `crumbkit/failure/v1` document on standard output,
///   and then from the last `code:` line on standard error. This holds with `SNAPSHOTS=overwrite` too, and the
///   check then writes no file for the example;
/// - a run that writes a `warning: crumbkit` line to standard error, which a
///   debug build writes for a recovery call that does not render. The check
///   then writes no file for the example;
/// - a run whose binary cannot start, such as a wrong path, naming the path
///   and the error;
/// - a run that does not end within 10 seconds, which the check stops, so a
///   stream command's example reads a finite input and never follows one;
/// - a run of an example whose words hold `--json`, for a leaf whose
///   document has crumbkit's envelope, that prints a line that is not a JSON
///   object whose `schema` is the document's id, or that prints a number of
///   lines other than one when the leaf declares a document of one result.
///   A leaf that declares a stream with `Leaf::stream` may print any number
///   of lines (ADR-0041). When the guide sets
///   [`JsonLayout::Pretty`](crumbkit::JsonLayout::Pretty), a leaf of one
///   result must instead print exactly one such object, indented by two
///   spaces, and a stream keeps one object per line (ADR-0052);
/// - two examples whose golden files have one name, naming both command
///   lines;
/// - an opt-out from [`Rule::Examples`] that names no command or skips no
///   example, and an [`Allow::example`](crate::Allow::example) that names no
///   example that the check runs;
/// - a directory of [`Examples::current_dir`] or a fixture of
///   [`Examples::fresh_dir`] that does not exist;
/// - a placeholder of [`Examples::redact`] that is not `[` and `]` around
///   capital letters and underscores, and an empty value;
/// - an input file that cannot be read, and an input file that names no
///   example of the guide, which the check never deletes, even with
///   `SNAPSHOTS=overwrite`, because only its author can write it again. An
///   example that an opt-out skips keeps its input file;
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
    let dir = Path::new(examples.goldens());
    let tool = command.get_name();
    let layout = guide.get_json_layout().map(|(layout, _)| layout);
    let mut problems = Vec::new();
    // The problems of runs that fail, which the message lists under a
    // heading of their own, because the tool's binary breaks them and the
    // command tree and golden files do not.
    let mut failures = Vec::new();
    let mut allowances = Allowances::new(examples.opt_outs(), &[Rule::Examples]);
    let visits = tree::walk(command);
    let form = guide.get_failure_form();
    if let Some(problem) = setup_problem(examples) {
        problems.push(problem);
        report(&problems, &failures);
        return;
    }
    let redactions = inputs::redactions(examples.redactions(), &mut problems);
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
            if allowances.excuses_example(&below, example.words()) {
                continue;
            }
            let name = example_golden_name(&line);
            if let Some((_, other)) = names.iter().find(|(known, _)| *known == name) {
                problems.push(format!(
                    "{line}: the example's golden file name `{name}` is also the name for the example `{other}`"
                ));
                continue;
            }
            names.push((name.clone(), line.clone()));
            let stdin = match read_stdin(dir, &name) {
                Ok(stdin) => stdin,
                Err(problem) => {
                    problems.push(format!("{line}: {problem}"));
                    continue;
                }
            };
            let run = run_example(examples, &args, stdin.as_deref(), &redactions, timeout);
            let (output, redactions) = match run {
                Ok(run) => run,
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
                lines::check_lines(leaf, layout, &line, &stdout, &mut failures);
            }
            let outputs = [redactions.redact(&stdout), redactions.redact(&stderr)];
            let stderr_name = compare_run(assert, dir, &line, &name, outputs, &mut problems);
            stderr_names.extend(stderr_name);
        }
    }
    let names: Vec<String> = names.into_iter().map(|(name, _)| name).collect();
    inputs::stale_stdin(tool, guide, dir, &mut problems);
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

/// Compares the redacted standard output and standard error in `outputs`
/// of the example `line` with the golden files `name` and its `.stderr`
/// file in `dir`, and returns the name of the `.stderr` file when the run
/// wrote to standard error.
fn compare_run(
    assert: &Assert,
    dir: &Path,
    line: &str,
    name: &str,
    outputs: [String; 2],
    problems: &mut Vec<String>,
) -> Option<String> {
    let [stdout, stderr] = outputs;
    let stderr_name = (!stderr.is_empty()).then(|| stderr_golden_name(name));
    if let Some(stderr_name) = &stderr_name {
        let path = dir.join(stderr_name);
        compare(assert, line, "standard error", stderr, &path, problems);
    }
    compare(assert, line, "output", stdout, &dir.join(name), problems);
    stderr_name
}

/// Runs the example `args` of `examples.binary` in fresh XDG directories,
/// in the working directory of `examples` or a new copy of its fresh
/// fixture, and with `stdin` on standard input, and returns its output with
/// `redactions` and the run's XDG directories and fresh directory, or a problem when the run cannot start or does not end
/// within `timeout`.
fn run_example(
    examples: &Examples<'_>,
    args: &[&str],
    stdin: Option<&[u8]>,
    redactions: &Redactions,
    timeout: Duration,
) -> Result<(Output, Redactions), String> {
    let started = Instant::now();
    let xdg = Xdg::new("example", examples.xdg_fixture())?;
    let work = WorkDir::new("example-dir", examples.working_dir())?;
    let place = Place {
        dir: work.path(),
        stdin,
    };
    let output = runs::run(
        examples.binary(),
        examples.variables(),
        &xdg,
        args,
        place,
        timeout,
    )?;
    if !output.status.success() && started.elapsed() >= timeout {
        return Err(format!(
            "the run did not end within {} seconds, so the check stopped it, and an example must end on its own",
            timeout.as_secs_f64()
        ));
    }
    Ok((output, inputs::with_run(redactions, &xdg, &work)))
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

/// Returns the name of the golden file of an example's standard error: the
/// name `name` of its output's golden file with `.stderr` in place of
/// `.txt`, such as `shelf-books-list.stderr`.
pub(crate) fn stderr_golden_name(name: &str) -> String {
    format!("{}.stderr", name.strip_suffix(".txt").unwrap_or(name))
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
