//! Tests for the example check's golden files of standard error, its
//! report of a failed run's failure, and the failure reader that reads it.

use std::fs;

use crumbkit::form::ReadFailure;
use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::failures::FailureReader;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of one example that prints `books` on standard output and
/// `note` on standard error.
const NOTED: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
    &["echo books; echo note >&2"],
    "Print a word and a note",
)])]);

/// The golden files of `NOTED`'s example.
const NOTED_OUT: &str = "sh-c-echo-books-echo-note-2.txt";
const NOTED_ERR: &str = "sh-c-echo-books-echo-note-2.stderr";

#[test]
fn overwrite_writes_an_examples_standard_error_to_a_stderr_golden_file_beside_its_output() {
    let dir = TempDir::new("examples-stderr-overwrite").unwrap();

    run(&NOTED, &examples(&dir), Action::Overwrite);

    assert_eq!(dir.names(), [NOTED_ERR, NOTED_OUT]);
    let note = fs::read_to_string(dir.path().join(NOTED_ERR)).unwrap();
    assert_eq!(note, "note\n");
}

#[test]
fn golden_files_that_hold_an_examples_standard_error_pass() {
    let dir = TempDir::new("examples-stderr-pass").unwrap();
    run(&NOTED, &examples(&dir), Action::Overwrite);

    run(&NOTED, &examples(&dir), Action::Verify);
}

#[test]
fn a_standard_error_without_a_golden_file_fails_the_check() {
    let dir = TempDir::new("examples-stderr-missing").unwrap();
    run(&NOTED, &examples(&dir), Action::Overwrite);
    #[expect(
        clippy::disallowed_methods,
        reason = "the test owns its temporary directory and removes the golden file it checks for"
    )]
    fs::remove_file(dir.path().join(NOTED_ERR)).unwrap();

    assert_eq!(
        panic_message(|| run(&NOTED, &examples(&dir), Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             sh -c 'echo books; echo note >&2': the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            dir.path().join(NOTED_ERR).display()
        )
    );
}

#[test]
fn a_standard_error_that_differs_from_its_golden_file_fails_the_check() {
    let dir = TempDir::new("examples-stderr-differs").unwrap();
    run(&NOTED, &examples(&dir), Action::Overwrite);
    dir.write(NOTED_ERR, "remark\n");

    let message = panic_message(|| run(&NOTED, &examples(&dir), Action::Verify)).unwrap();

    let heading = format!(
        "the command tree breaks crumbkit's example rules:\n\
         sh -c 'echo books; echo note >&2': the standard error differs from the golden file `{}`:\n",
        dir.path().join(NOTED_ERR).display()
    );
    assert!(message.starts_with(&heading), "{message}");
    assert!(message.contains("remark"), "{message}");
}

#[test]
fn a_stderr_golden_file_of_an_example_that_writes_nothing_to_standard_error_is_stale() {
    const QUIET: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo books"], "Print a word")])]);
    let dir = TempDir::new("examples-stderr-stale").unwrap();
    run(&QUIET, &examples(&dir), Action::Overwrite);
    dir.write("sh-c-echo-books.stderr", "note\n");

    let message = panic_message(|| run(&QUIET, &examples(&dir), Action::Verify)).unwrap();
    run(&QUIET, &examples(&dir), Action::Overwrite);

    assert_eq!(
        message,
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the golden file `{}` belongs to no checked example that writes to standard error, and `SNAPSHOTS=overwrite` deletes it",
            dir.path().join("sh-c-echo-books.stderr").display()
        )
    );
    assert_eq!(dir.names(), ["sh-c-echo-books.txt"]);
}

#[test]
fn an_example_that_writes_a_crumbkit_warning_fails_the_check_and_overwrite_writes_no_file() {
    const WARNS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &["echo books; echo 'warning: sh left out a recovery' >&2"],
        "Warn",
    )])]);
    let dir = TempDir::new("examples-stderr-warning").unwrap();

    assert_eq!(
        panic_message(|| run(&WARNS, &examples(&dir), Action::Overwrite)).unwrap(),
        "these runs of the tool's examples fail:\n\
         sh -c 'echo books; echo '\\''warning: sh left out a recovery'\\'' >&2': \
         the run writes a warning to standard error: warning: sh left out a recovery"
    );
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn an_example_that_writes_the_tools_own_warning_passes_the_check() {
    const WARNS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &["echo books; echo 'warning: sh cache is stale' >&2"],
        "Warn",
    )])]);
    let dir = TempDir::new("examples-stderr-own-warning").unwrap();

    assert_eq!(
        panic_message(|| run(&WARNS, &examples(&dir), Action::Overwrite)),
        None
    );
}

#[test]
fn an_example_that_fails_names_the_code_and_steps_of_the_failure_document_it_prints() {
    const FAILS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &[r#"echo '{"schema":"sh/failure/v1","code":"sh::books::gone","message":"m","recovery":{"kind":"run","call":"sh -c ls"},"alternatives":[{"kind":"retry"}]}'; exit 1"#],
        "Fail",
    )])]);
    let dir = TempDir::new("examples-failure-document").unwrap();

    let message = panic_message(|| run(&FAILS, &examples(&dir), Action::Overwrite)).unwrap();

    assert!(
        message.ends_with(
            ": an example must exit with code 0, and the run ends with exit status: 1 \
             and fails with sh::books::gone, whose steps are: run `sh -c ls`; retry"
        ),
        "{message}"
    );
}

#[test]
fn an_example_that_fails_in_text_names_the_code_of_its_last_code_line() {
    const FAILS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &["printf 'error: gone\\n  code:  sh::books::gone\\n' >&2; exit 1"],
        "Fail",
    )])]);
    let dir = TempDir::new("examples-failure-text").unwrap();

    let message = panic_message(|| run(&FAILS, &examples(&dir), Action::Overwrite)).unwrap();

    assert!(
        message.ends_with(
            ": an example must exit with code 0, and the run ends with exit status: 1 \
             and fails with sh::books::gone after writing this to standard error:\n\
             error: gone\n  code:  sh::books::gone\n"
        ),
        "{message}"
    );
}

/// A failure reader that reads a last line on standard error of `ERR` and
/// a code.
struct LineReader;

impl FailureReader for LineReader {
    fn read(&self, _out: &str, err: &str) -> Vec<ReadFailure> {
        err.lines()
            .last()
            .and_then(|line| line.strip_prefix("ERR "))
            .map(|code| ReadFailure::new(code.to_owned(), Vec::new()))
            .into_iter()
            .collect()
    }
}

#[test]
fn an_example_that_fails_names_the_code_that_the_failure_reader_reads() {
    const FAILS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &["echo 'ERR sh::books::gone' >&2; exit 1"],
        "Fail",
    )])]);
    let dir = TempDir::new("examples-failure-reader").unwrap();

    let message = panic_message(|| {
        run(
            &FAILS,
            &examples(&dir).failure_reader(&LineReader),
            Action::Overwrite,
        );
    })
    .unwrap();

    assert!(
        message.contains(
            ": an example must exit with code 0, and the run ends with exit status: 1 \
             and fails with sh::books::gone after writing"
        ),
        "{message}"
    );
}
