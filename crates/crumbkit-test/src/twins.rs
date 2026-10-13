//! The twin check, which runs a command line that a tool refuses beside one
//! that differs from it by one word and succeeds.

use std::path::Path;
use std::process::Output;

use crumbkit::Spelling;
use crumbkit::help::Guide;

use crate::examples::Examples;
use crate::failures;
use crate::guide::command_line;
use crate::recoveries::Failing;
use crate::runs::{self, Dir, Place, Ran, TIMEOUT, WorkDir, Xdg};

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
/// `examples` that differ by one word, for the tool whose guide is `guide`, checks that `refused` fails with its
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
/// admitted line that runs another leaf of the guide than the refused line,
/// or no leaf, runs another command. The leaf of a line is the longest leaf
/// path that its words start with, and a refused line that runs no leaf,
/// such as a misspelt command, keeps its twin. A code that no single word
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
/// ```no_run
/// use crumbkit::Spelling;
/// use crumbkit::help::Guide;
/// use crumbkit_test::{Examples, Failing};
///
/// const GUIDE: Guide = Guide::new();
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let output = crumbkit_test::check_twin(
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
/// runs another leaf than `refused`, when `refused` exits with code 0, fails with
/// another code or prints no code that the check can read, when `admitted` exits with a code other than 0, or when a
/// run cannot start or does not end within 10 seconds. The message names
/// each line as help prints it, with the binary's file name first.
#[expect(
    clippy::must_use_candidate,
    reason = "a test that compares no output with golden files calls the check only for its assertions"
)]
pub fn check_twin(
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: &[&str],
) -> Output {
    match twin(guide, examples, spelling, refused, admitted) {
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

/// Runs the two lines of [`check_twin`] and returns the output of
/// `refused`, or the problem when a rule is broken.
pub(super) fn twin(
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: &[&str],
) -> Result<Output, String> {
    let tool = tool_name(examples);
    let refused_line = command_line(&tool, refused.words());
    let admitted_line = command_line(&tool, admitted);
    if !words::one_word_apart(refused.words(), admitted) {
        return Err(format!(
            "the refused line `{refused_line}` and the admitted line `{admitted_line}` must differ by one word added, removed or replaced"
        ));
    }
    if let Some(word) = words::skipping_word(refused.words(), admitted) {
        return Err(format!(
            "the admitted line `{admitted_line}` adds `{word}`, which makes the tool answer without running the command, so the twin proves nothing about the refused line `{refused_line}`. {UNTWINNED}"
        ));
    }
    if let Some(path) = words::leaf(guide, refused.words()) {
        let other = words::leaf(guide, admitted);
        if other != Some(path) {
            let runs = other.map_or_else(
                || "runs no command of the guide".to_owned(),
                |other| format!("runs `{}`", other.join(" ")),
            );
            return Err(format!(
                "the refused line `{refused_line}` runs the command `{}`, and the admitted line `{admitted_line}` {runs}, so the twin proves nothing about the refused command. A code that no word of the command causes needs an opt-out made with Allow::untwinned",
                path.join(" ")
            ));
        }
    }
    let output = refuse(guide, examples, spelling, refused)?;
    let dir = refused.work_dir(examples.working_dir());
    let stdin = refused.stdin_bytes();
    let fixture = refused.fixture(examples.xdg_fixture());
    let other = run(examples, admitted, (dir, stdin), fixture, &admitted_line)?;
    if !other.status.success() {
        return Err(format!(
            "the admitted line `{admitted_line}` must exit with code 0, and it ends with {} after writing this to standard error:\n{}",
            other.status,
            String::from_utf8_lossy(&other.stderr)
        ));
    }
    Ok(output)
}

/// The end of the problem of a twin whose admitted line skips the
/// command's work.
const UNTWINNED: &str =
    "A code that no word of a command line causes needs an opt-out made with Allow::untwinned";

/// Runs `refused`, checks that it fails with its code, and returns its
/// output, or the problem when it exits with code 0, fails with another
/// code or prints no code that the check can read.
pub(super) fn refuse(
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
) -> Result<Output, String> {
    let reader = failures::reader(guide, examples)?;
    let refused_line = command_line(&tool_name(examples), refused.words());
    let dir = refused.work_dir(examples.working_dir());
    let stdin = refused.stdin_bytes();
    let code = spelling.spell(refused.code());
    let fixture = refused.fixture(examples.xdg_fixture());
    let output = run(
        examples,
        refused.words(),
        (dir, stdin),
        fixture,
        &refused_line,
    )?;
    if output.status.success() {
        return Err(format!(
            "the refused line `{refused_line}` must fail with {code}, and it exits with code 0"
        ));
    }
    // The first failure is the one the refused line fails with.
    let printed = failures::read_failures(reader, &output.stdout, &output.stderr)
        .first()
        .map(|failure| failure.code().to_owned());
    match printed {
        Some(printed) if printed == code => Ok(output),
        Some(printed) => Err(format!(
            "the refused line `{refused_line}` must fail with {code}, and it fails with {printed}"
        )),
        None => Err(format!(
            "the refused line `{refused_line}` must fail with {code}, and it ends with {} and prints no code that the check can read. A tool that prints failures in a form of its own implements FailureForm::read and sets the form with Guide::failure_form, or implements FailureReader and sets it with Examples::failure_reader",
            output.status
        )),
    }
}

/// Runs `words` with fresh XDG directories that start as copies of
/// `fixture`, and with the working directory and standard input of `start`,
/// a new copy for a fresh fixture, and returns the problem naming `line`
/// when the run cannot start or does not end.
fn run(
    examples: &Examples<'_>,
    words: &[&str],
    start: (Option<Dir<'_>>, Option<&[u8]>),
    fixture: Option<&str>,
    line: &str,
) -> Result<Output, String> {
    let (dir, stdin) = start;
    let ran = Xdg::new("twin", fixture)
        .and_then(|xdg| {
            let work = WorkDir::new("twin-dir", dir)?;
            let place = Place {
                dir: work.path(),
                stdin,
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
        .map_err(|problem| format!("`{line}`: {problem}"))?;
    let Ran::Exited(output) = ran else {
        return Err(format!(
            "`{line}` did not end within {} seconds, so the check stopped it",
            TIMEOUT.as_secs()
        ));
    };
    Ok(output)
}
