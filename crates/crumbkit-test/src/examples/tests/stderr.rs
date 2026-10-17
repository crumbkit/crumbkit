//! Tests for the example check's golden files of standard error, its
//! report of a failed run's failure, and the failure form and failure
//! reader that read it.

use std::fs;

use crumbkit::Document;
use crumbkit::form::{FailureForm, PreparedFailure, ReadFailure};
use crumbkit::help::{Example, Guide, Leaf};
use schemars::{Schema, json_schema};
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

/// A failure form whose text is one line, `ERR` and the code, and which
/// reads that line back.
struct LineForm;

impl FailureForm for LineForm {
    fn document(&self) -> Document<()> {
        Document::new("sh", "error", 1, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        serde_json::json!({ "error": failure.code() })
    }

    fn read(&self, _out: &str, err: &str) -> Option<ReadFailure> {
        let code = err.lines().last()?.strip_prefix("ERR ")?;
        Some(ReadFailure::new(code.to_owned(), Vec::new()))
    }
}

static FORM: LineForm = LineForm;

/// The examples of a tool with `LineForm`, whose one example fails.
const FAILS_IN_FORM: Guide = Guide::new().failure_form(&FORM).leaves(&[Leaf::new(&["-c"])
    .examples(&[Example::new(
        &["echo 'ERR sh::books::gone' >&2; exit 1"],
        "Fail",
    )])]);

#[test]
fn an_example_that_fails_names_the_code_that_the_failure_form_reads() {
    let dir = TempDir::new("examples-failure-form").unwrap();

    let message = panic_message(|| {
        run(&FAILS_IN_FORM, &examples(&dir), Action::Overwrite);
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

/// A failure reader that reads the line `ERR` and a code that `LineForm`
/// prints.
struct LineReader;

impl FailureReader for LineReader {
    fn read(&self, out: &str, err: &str) -> Vec<ReadFailure> {
        LineForm.read(out, err).into_iter().collect()
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

#[test]
fn a_guide_with_a_failure_form_and_examples_with_a_failure_reader_fail_the_check() {
    let dir = TempDir::new("examples-form-and-reader").unwrap();

    let message = panic_message(|| {
        run(
            &FAILS_IN_FORM,
            &examples(&dir).failure_reader(&LineReader),
            Action::Overwrite,
        );
    });

    assert_eq!(
        message.unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the guide sets a failure form with Guide::failure_form and the examples set a \
         failure reader with Examples::failure_reader, and a tool reads its failures \
         through only one of them"
    );
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}
