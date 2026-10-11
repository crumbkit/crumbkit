//! The example check, which runs every example in a tool's guide against the
//! tool's built binary and pins each run's standard output in a golden file.

use std::path::Path;
use std::process::Output;

use clap::Command;
use crumbkit::help::Guide;
use snapbox::assert::{Action, DEFAULT_ACTION_ENV};
use snapbox::{Assert, cmd};

use crate::goldens::{compare, remove_stale};
use crate::guide::command_line;
use crate::temp::TempDir;
use crate::tree;

#[cfg(test)]
mod tests;

/// The XDG base directory variables that each run sets, with the name of the
/// fresh directory that each one points at.
const XDG: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_STATE_HOME", "state"),
    ("XDG_CACHE_HOME", "cache"),
];

/// The settings of [`check_examples`]: the tool's built binary, the
/// directory of the golden files, the environment of each run and the
/// exempt commands.
///
/// A tool declares them as a constant in an integration test of the
/// binary's own package, because cargo sets `CARGO_BIN_EXE_<name>`, the
/// path of the built binary, only for that package's integration tests and
/// benchmarks. An integration test can import only from a library, so the
/// package needs a library target that exports the clap command and the
/// guide, and the binary calls that library:
///
/// ```
/// use crumbkit_test::Examples;
///
/// // In `tests/examples.rs`, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples = Examples::new("/path/to/shelf", "tests/golden/examples")
///     .env(&[("SHELF_DATA", "tests/fixtures/books.json")])
///     .exempt(&["books delete"]);
/// ```
///
/// cargo runs those tests, and therefore the binary, from the package's
/// manifest directory, so a relative path in `dir` or in an `.env` value
/// resolves from there.
#[derive(Clone, Copy, Debug)]
pub struct Examples {
    /// The path of the tool's built binary.
    binary: &'static str,
    /// The directory of the golden files, which holds only example golden
    /// files.
    dir: &'static str,
    /// The environment variables that each run sets, as name and value.
    env: &'static [(&'static str, &'static str)],
    /// The paths of the commands whose examples the check skips, each
    /// below the root.
    exempt: &'static [&'static str],
}

impl Examples {
    /// Returns the settings that run the binary at `binary` and keep the
    /// golden files in `dir`, with no environment variables of the tool's
    /// own and no exempt command.
    ///
    /// `dir` holds only example golden files, because the check counts every
    /// `.txt` file in it that no checked example produces as stale.
    #[must_use]
    pub const fn new(binary: &'static str, dir: &'static str) -> Self {
        Self {
            binary,
            dir,
            env: &[],
            exempt: &[],
        }
    }

    /// Sets the environment variables that each run sets, each a name and a
    /// value, such as `("SHELF_DATA", "tests/fixtures/books.json")`, so a
    /// tool can point its binary at fixture data or a fake server. The XDG
    /// directories that the check sets win over a variable of the same name.
    #[must_use]
    pub const fn env(mut self, env: &'static [(&'static str, &'static str)]) -> Self {
        self.env = env;
        self
    }

    /// Sets the commands whose examples the check skips, each a command's
    /// path below the root, with its names joined by single spaces, such as
    /// `"books list"`. The check skips the examples of each named command and
    /// of every command below it.
    #[must_use]
    pub const fn exempt(mut self, exempt: &'static [&'static str]) -> Self {
        self.exempt = exempt;
        self
    }
}

/// Runs every example in the leaves of `guide` against the binary that
/// `examples` names and checks that each run succeeds and prints the
/// standard output in its golden file, and panics, as a failed test
/// assertion does, naming every problem it finds.
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
/// command that `examples` exempts and of every command below it. It runs
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
/// The golden file of an example holds the run's standard output. Its name
/// joins the tool's name, the leaf's path and the example's words with
/// dashes and ends with `.txt`, such as `shelf-books-list.txt`,
/// `shelf-books-list-all.txt` and `shelf-books-show-2-5.txt`. Within each
/// part, every run of characters other than ASCII letters, digits, `.` and
/// `_` becomes one dash, and a dash at the start or the end of the part is
/// dropped, so `--all` becomes `all`. A part with no letter, digit, `.` or
/// `_`, such as `--`, is left out.
///
/// The check compares each output with snapbox, as other golden files are
/// compared (ADR-0004). With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file, rewrites each file whose content differs
/// and deletes each stale file. It finds:
///
/// - a run that exits with a code other than 0 or writes to standard error,
///   naming the example's command line, the exit status and the standard
///   error, because an example shows a run that works. This holds with
///   `SNAPSHOTS=overwrite` too, and the check then writes no file for the
///   example;
/// - a run whose binary cannot start, such as a wrong path, naming the path
///   and the error;
/// - two examples whose golden files have one name, naming both command
///   lines;
/// - an exemption that names no command;
///
/// and, without `SNAPSHOTS=overwrite`:
///
/// - an example without a golden file;
/// - an output that differs from its golden file, with snapbox's report of
///   both texts;
/// - a stale golden file: a `.txt` file in the directory that no checked
///   example produces, such as the file of a removed or exempt example.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::help::{Example, Guide, Leaf};
/// use crumbkit_test::Examples;
///
/// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
///     .examples(&[Example::new(&["--all"], "Print every book")])]);
///
/// // In `tests/examples.rs`, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let shelf = Command::new("shelf")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_examples(&shelf, &GUIDE, &EXAMPLES);
/// ```
///
/// # Panics
///
/// Panics when a run, a golden file or an exemption breaks a rule above, or
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
pub fn check_examples(command: &Command, guide: &Guide, examples: &Examples) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, guide, examples, &assert);
}

/// Runs [`check_examples`] with `assert`, whose action decides whether the
/// check compares or overwrites, so a test chooses the action without
/// setting `SNAPSHOTS`.
fn check(command: &Command, guide: &Guide, examples: &Examples, assert: &Assert) {
    let overwrite = assert.selected_action() == Action::Overwrite;
    let dir = Path::new(examples.dir);
    let tool = command.get_name();
    let mut problems = Vec::new();
    // The problems of runs that fail, which the message lists under a
    // heading of their own, because the tool's binary breaks them and the
    // command tree and golden files do not.
    let mut failures = Vec::new();
    let visits = tree::walk(command, examples.exempt, &mut problems);
    // Each golden file name with the command line of the example that has
    // it.
    let mut names: Vec<(String, String)> = Vec::new();
    for leaf in guide.get_leaves() {
        let path = leaf.path();
        let below = path.join(" ");
        if !visits.iter().any(|visit| visit.below() == below) {
            continue;
        }
        for example in leaf.get_examples() {
            let args: Vec<&str> = path.iter().chain(example.words()).copied().collect();
            let text = std::iter::once(tool)
                .chain(path.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            let line = command_line(&text, example.words());
            let name = file_name(tool, path, example.words());
            if let Some((_, other)) = names.iter().find(|(known, _)| *known == name) {
                problems.push(format!(
                    "{line}: the example's golden file name `{name}` is also the name for the example `{other}`"
                ));
                continue;
            }
            names.push((name.clone(), line.clone()));
            let output = match run(examples, &args) {
                Ok(output) => output,
                Err(problem) => {
                    failures.push(format!("{line}: {problem}"));
                    continue;
                }
            };
            if !output.status.success() || !output.stderr.is_empty() {
                let mut problem = format!(
                    "{line}: an example must exit with code 0 and write nothing to standard error, and the run ends with {}",
                    output.status
                );
                if !output.stderr.is_empty() {
                    problem.push_str(" after writing this to standard error:\n");
                    problem.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                failures.push(problem);
                continue;
            }
            // RUST-10: the check compares the output as UTF-8 text, as the
            // help golden check compares pages. It writes each byte of the
            // output that is not valid UTF-8 as U+FFFD, so it cannot pin
            // binary output. A comparison of `Data::binary` replaces it when
            // a tool's examples print bytes.
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
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
    remove_stale(dir, &names, overwrite, "checked example", &mut problems);
    report(&problems, &failures);
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

/// Runs the binary of `examples` once with `args`, a cleared environment,
/// the variables of [`Examples::env`] and fresh XDG directories, and returns
/// its output, or the problem when the XDG directories cannot be created or
/// the binary cannot start.
///
/// The check owns the process effects of each run, and runs it through
/// snapbox's `cmd` module, which captures standard output and standard
/// error.
fn run(examples: &Examples, args: &[&str]) -> Result<Output, String> {
    let xdg = TempDir::new("example")
        .map_err(|error| format!("the run's XDG directories cannot be created: {error}"))?;
    let mut command = cmd::Command::new(examples.binary).args(args).env_clear();
    for &(name, value) in examples.env {
        command = command.env(name, value);
    }
    for (variable, name) in XDG {
        let path = xdg
            .create_dir(name)
            .map_err(|error| format!("the run's XDG directories cannot be created: {error}"))?;
        command = command.env(variable, path);
    }
    command
        .output()
        .map_err(|error| format!("the binary `{}` cannot start: {error}", examples.binary))
}

/// Returns the name of the golden file of the example that runs `tool` with
/// `path` and `words`: each of them as [`part`] writes it, left out when
/// empty, joined by dashes, with `.txt` appended.
fn file_name(tool: &str, path: &[&str], words: &[&str]) -> String {
    let parts: Vec<String> = std::iter::once(tool)
        .chain(path.iter().copied())
        .chain(words.iter().copied())
        .map(part)
        .filter(|part| !part.is_empty())
        .collect();
    format!("{}.txt", parts.join("-"))
}

/// Returns `word` with each run of characters other than ASCII letters,
/// digits, `.` and `_` written as one dash, and without a dash at its start
/// or its end, so `--all` becomes `all` and `title, year` becomes
/// `title-year`.
fn part(word: &str) -> String {
    word.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_'))
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
