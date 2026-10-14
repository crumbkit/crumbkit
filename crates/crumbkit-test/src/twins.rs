//! The twin check, which runs a command line that a tool refuses beside one
//! that differs from it by one word and succeeds.

use std::path::Path;
use std::process::Output;

use clap::Command;
use crumbkit::Spelling;
use crumbkit::help::Guide;

use crate::examples::Examples;
use crate::exits;
use crate::failures;
use crate::guide::command_line;
use crate::recoveries::Failing;
use crate::runs::{self, Place, Ran, TIMEOUT, WorkDir, Xdg, env_note};

mod coverage;
mod words;

pub use coverage::{Twin, Twins, check_twins};

// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// Runs `refused` and `admitted`, two command lines of the binary of
/// `examples` that differ by one word, for the tool whose unbuilt command
/// tree is `command` and whose guide is `guide`, checks that `refused` fails with its
/// code and that `admitted` exits with code 0, and returns the output of
/// `refused`, so the test can compare it with its golden files. It panics,
/// as a failed test assertion does, when a rule is broken.
///
/// A test of a refusal proves only that the tool fails. Its twin proves
/// that the one word it changes is the reason, because the admitted line
/// without that change succeeds.
///
/// Two kinds of twin prove nothing, and the check refuses them before it
/// runs either line. An admitted line that adds `-h`, `--help`, `-V` or
/// `--version`, or `help` as its first word, or puts one of them in place
/// of a word, makes the tool answer without running the command. An
/// admitted line that runs another command than the refused line, or no
/// command, proves nothing about the refused command. The check reads the
/// command of a line from `command` with [`crumbkit::command_path`], which
/// skips options and their values, global options before the path
/// included, and resolves aliases. A line runs the command it names when
/// that command has no subcommands. A refused line that runs no command,
/// such as the misspelt `books lst`, needs an admitted line that runs one,
/// such as `books list`. A code that no single word
/// causes needs an opt-out made with [`Allow::untwinned`](crate::Allow::untwinned).
///
/// Both runs use the environment of [`Examples::env`], the working
/// directory and standard input of `refused`, or the directory of
/// [`Examples::current_dir`] when `refused` sets none, or a new copy each
/// of the fixture of [`Examples::fresh_dir`], and fresh XDG directories of
/// their own, as [`check_examples`](crate::check_examples) runs an example.
/// The check reads the refused line's code, spelled with `spelling`. When
/// the guide sets a form with `Guide::failure_form`, it reads the code
/// through the form's
/// [`FailureForm::read`](crumbkit::form::FailureForm::read) first, and when
/// `examples` sets a [`FailureReader`](crate::FailureReader), through that
/// reader first. Otherwise, or when neither reads a failure, it reads the
/// code from a `crumbkit/failure/v1` document on standard output, or from
/// the first entry of a batch's `failures`, and then from the last `code:`
/// line on standard error.
///
/// The refused line must exit with the code that
/// [`Guide::failure_exit`](crumbkit::help::Guide::failure_exit) gives its
/// failure: the code of the guide's [`Exit`](crumbkit::help::Exit) that
/// lists its code with [`Exit::failures`](crumbkit::help::Exit::failures),
/// the code of a refused command line for `crumbkit::cli::usage`, which is
/// 2 unless an exit lists it, and 1 for any other code. A command whose
/// leaf declares [`Leaf::passes_on_exit`](crumbkit::help::Leaf::passes_on_exit)
/// may end an unlisted failure with any code but 0. A refused line whose
/// code no exit lists never exits with the code of an exit that lists
/// failure codes.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::Spelling;
/// use crumbkit::help::Guide;
/// use crumbkit_test::{Examples, Failing};
///
/// const GUIDE: Guide = Guide::new();
/// let command = Command::new("shelf")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let output = crumbkit_test::check_twin(
///     &command,
///     &GUIDE,
///     &EXAMPLES,
///     Spelling::COLONS,
///     &Failing::new(&crumbkit::codes::CLI_USAGE, &["books", "list", "--limit", "0"]),
///     &["books", "list", "--limit", "1"],
/// );
/// assert!(output.stdout.is_empty());
/// ```
///
/// # Panics
///
/// Panics when the guide sets a failure form and `examples` sets a failure
/// reader, when the two lines differ by more or less than one word added,
/// removed or replaced, when `admitted` adds a help or version word or
/// runs another command than `refused`, when neither line runs a command, when `refused` exits with code 0, fails with
/// another code, exits with a code that breaks the rule of the guide's exits
/// above or prints no code that the check can read, when `admitted` exits with a code other than 0, or when a
/// run cannot start or does not end within 10 seconds. The message names
/// each line as help prints it, with the binary's file name first.
#[expect(
    clippy::must_use_candidate,
    reason = "a test that compares no output with golden files calls the check only for its assertions"
)]
pub fn check_twin(
    command: &Command,
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: &[&str],
) -> Output {
    match twin(
        command,
        guide,
        examples,
        spelling,
        refused,
        Admitted::Words(admitted),
    ) {
        Ok(output) => output,
        Err(problem) => {
            #[expect(
                clippy::panic,
                reason = "a check reports a broken rule by panicking, as a failed test assertion does"
            )]
            {
                panic!("{problem}")
            }
        }
    }
}

/// Returns the file name of the binary of `examples` without its
/// extension, which names the tool in a command line.
pub(super) fn tool_name(examples: &Examples<'_>) -> String {
    Path::new(examples.binary())
        .file_stem()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// The admitted line of a twin.
#[derive(Clone, Copy, Debug)]
pub(super) enum Admitted<'a> {
    /// The words of a line that differs from the refused line by one word,
    /// which runs with the refused line's variables.
    Words(&'a [&'a str]),
    /// The refused line's words, run without the refused line's own entry
    /// for the variable of this name.
    Without(&'a str),
}

/// Runs the two lines of [`check_twin`] and returns the output of
/// `refused`, or the problem when a rule is broken.
pub(super) fn twin(
    command: &Command,
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: Admitted<'_>,
) -> Result<Output, String> {
    let tool = tool_name(examples);
    let refused_line = named(&tool, refused.words(), refused.own_env());
    let (words, env, admitted_line) = match admitted {
        Admitted::Words(words) => {
            let admitted_line = named(&tool, words, refused.own_env());
            one_word(
                command,
                guide,
                refused.words(),
                words,
                &refused_line,
                &admitted_line,
            )?;
            (words, refused.own_env().to_vec(), admitted_line)
        }
        Admitted::Without(name) => {
            let env = refused.own_env();
            let Some(&(_, value)) = env.iter().find(|&&(own, _)| own == name) else {
                return Err(format!(
                    "the variable twin of the refused line {refused_line} names {name}, which the refused line neither sets nor removes"
                ));
            };
            let rest: Vec<_> = env
                .iter()
                .copied()
                .filter(|&(own, _)| own != name)
                .collect();
            let without = value.map_or_else(
                || format!("without removing {name}"),
                |value| format!("without {name}={value}"),
            );
            let note = env_note(&rest);
            let note = note.strip_suffix(')').map_or_else(
                || format!(" ({without})"),
                |note| format!("{note}, {without})"),
            );
            let line = format!("`{}`{note}", command_line(&tool, refused.words()));
            (refused.words(), rest, line)
        }
    };
    let output = refuse(command, guide, examples, spelling, refused)?;
    let other = run(examples, refused, words, &env, &admitted_line)?;
    if !other.status.success() {
        return Err(format!(
            "the admitted line {admitted_line} must exit with code 0, and it ends with {} after writing this to standard error:\n{}",
            other.status,
            String::from_utf8_lossy(&other.stderr)
        ));
    }
    Ok(output)
}

/// Returns the command line of `words` in backticks, with the note of the
/// variables of `env` when there are any, such as
/// `` `tool books list` (with DEPTH=1) ``.
pub(super) fn named(tool: &str, words: &[&str], env: &[(&str, Option<&str>)]) -> String {
    format!("`{}`{}", command_line(tool, words), env_note(env))
}

/// Returns the problem of a twin whose lines, named `refused_line` and
/// `admitted_line`, do not differ by one word, whose admitted line adds a
/// word that skips the command's work, or whose admitted line runs another
/// command of `command`, the tool's tree, than the refused line, or no
/// command when the refused line runs none. `guide` is the tool's guide,
/// whose declarations say which words name a command.
fn one_word(
    command: &Command,
    guide: &Guide,
    refused: &[&str],
    admitted: &[&str],
    refused_line: &str,
    admitted_line: &str,
) -> Result<(), String> {
    if !words::one_word_apart(refused, admitted) {
        return Err(format!(
            "the refused line {refused_line} and the admitted line {admitted_line} must differ by one word added, removed or replaced"
        ));
    }
    if let Some(word) = words::skipping_word(refused, admitted) {
        return Err(format!(
            "the admitted line {admitted_line} adds `{word}`, which makes the tool answer without running the command, so the twin proves nothing about the refused line {refused_line}. A code that no word of a command line causes needs an opt-out made with Allow::untwinned"
        ));
    }
    let named = words::named(command, guide, refused);
    let other = words::named(command, guide, admitted);
    if !named.runs && !other.runs {
        return Err(format!(
            "the refused line {refused_line} runs no command, and the admitted line {admitted_line} runs none either, so the twin proves nothing about a command. A twin of a line that runs no command admits a line that runs one"
        ));
    }
    if named.runs && other.path != named.path {
        let runs = if other.runs {
            format!("runs `{}`", other.path.join(" "))
        } else {
            "runs no command".to_owned()
        };
        return Err(format!(
            "the refused line {refused_line} runs the command `{}`, and the admitted line {admitted_line} {runs}, so the twin proves nothing about the refused command. A code that no word of the command causes needs an opt-out made with Allow::untwinned",
            named.path.join(" ")
        ));
    }
    Ok(())
}

/// Runs `refused`, checks that it fails with its code, and returns its
/// output, or the problem when it exits with code 0, fails with another
/// code or prints no code that the check can read.
pub(super) fn refuse(
    command: &Command,
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
) -> Result<Output, String> {
    let reader = failures::reader(guide, examples)?;
    let refused_line = named(&tool_name(examples), refused.words(), refused.own_env());
    // A leaf's renames decide the code that a run of its command prints, so
    // the code is spelled for the command that the refused line names.
    let path = crumbkit::command_path(command, guide, refused.words());
    let leaf_path: Vec<&str> = path.iter().map(String::as_str).collect();
    let code = spelling.spell(guide.printed(&leaf_path, refused.code()));
    let output = run(
        examples,
        refused,
        refused.words(),
        refused.own_env(),
        &refused_line,
    )?;
    if output.status.success() {
        return Err(format!(
            "the refused line {refused_line} must fail with {code}, and it exits with code 0"
        ));
    }
    // The first failure is the one the refused line fails with.
    let printed = failures::read_failures(reader, &output.stdout, &output.stderr)
        .first()
        .map(|failure| failure.code().to_owned());
    match printed {
        Some(printed) if printed == code => {
            match exits::exit_problem(guide, spelling, &path, &printed, output.status) {
                Some(problem) => Err(format!("the refused line {refused_line} {problem}")),
                None => Ok(output),
            }
        }
        Some(printed) => Err(format!(
            "the refused line {refused_line} must fail with {code}, and it fails with {printed}"
        )),
        None => Err(format!(
            "the refused line {refused_line} must fail with {code}, and it ends with {} and prints no code that the check can read. A tool that prints failures in a form of its own implements FailureForm::read and sets the form with Guide::failure_form, or implements FailureReader and sets it with Examples::failure_reader",
            output.status
        )),
    }
}

/// Runs `words` with the variables of `env`, in the working directory,
/// with the standard input and in fresh XDG directories that start as
/// copies of the fixture of `refused`, and returns its output, or the
/// problem, which names the run by `line`, when the run cannot start or
/// does not end.
fn run(
    examples: &Examples<'_>,
    refused: &Failing<'_>,
    words: &[&str],
    env: &[(&str, Option<&str>)],
    line: &str,
) -> Result<Output, String> {
    let fixture = refused.fixture(examples.xdg_fixture());
    let dir = refused.work_dir(examples.working_dir());
    let ran = Xdg::new("twin", fixture)
        .and_then(|xdg| {
            let work = WorkDir::new("twin-dir", dir)?;
            let place = Place {
                dir: work.path(),
                stdin: refused.stdin_bytes(),
                env,
            };
            runs::run(
                examples.binary(),
                examples.variables(),
                &xdg,
                words,
                place,
                TIMEOUT,
            )
        })
        .map_err(|problem| format!("{line}: {problem}"))?;
    let Ran::Exited(output) = ran else {
        return Err(format!(
            "{line} did not end within {} seconds, so the check stopped it",
            TIMEOUT.as_secs()
        ));
    };
    Ok(output)
}
