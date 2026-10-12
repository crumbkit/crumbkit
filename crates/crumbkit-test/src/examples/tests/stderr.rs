//! Tests for the example check's golden files of standard error, its
//! report of a failed run's failure, and the failure form that `Examples`
//! names.

use std::fs;

use crumbkit::Document;
use crumbkit::form::{FailureForm, PreparedFailure, ReadFailure};
use crumbkit::help::{Example, Guide, Leaf};
use schemars::{Schema, json_schema};
use snapbox::assert::Action;

use super::{examples, run};
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
        &["echo books; echo 'warning: crumbkit left out a recovery' >&2"],
        "Warn",
    )])]);
    let dir = TempDir::new("examples-stderr-warning").unwrap();

    assert_eq!(
        panic_message(|| run(&WARNS, &examples(&dir), Action::Overwrite)).unwrap(),
        "these runs of the tool's examples fail:\n\
         sh -c 'echo books; echo '\\''warning: crumbkit left out a recovery'\\'' >&2': \
         the run writes a warning to standard error: warning: crumbkit left out a recovery"
    );
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn an_example_that_fails_names_the_code_and_steps_of_the_failure_document_it_prints() {
    const FAILS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &[r#"echo '{"schema":"crumbkit/failure/v1","code":"sh::books::gone","message":"m","recovery":{"kind":"run","call":"sh -c ls"},"alternatives":[{"kind":"retry"}]}'; exit 1"#],
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

/// A form whose document id differs from `LineForm`'s.
struct OtherForm;

impl FailureForm for OtherForm {
    fn document(&self) -> Document<()> {
        Document::new("sh", "fault", 1, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        LineForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        LineForm.json(failure)
    }
}

static FORM: LineForm = LineForm;
static OTHER: OtherForm = OtherForm;

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
        run(
            &FAILS_IN_FORM,
            &examples(&dir).failure_form(&FORM),
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
fn a_guide_and_examples_that_name_different_failure_forms_fail_the_check() {
    const QUIET: Guide = Guide::new()
        .failure_form(&FORM)
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo books"], "Print")])]);
    const NO_FORM: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo books"], "Print")])]);
    let dir = TempDir::new("examples-form-mismatch").unwrap();

    let none = panic_message(|| run(&QUIET, &examples(&dir), Action::Overwrite));
    let other = panic_message(|| {
        run(
            &QUIET,
            &examples(&dir).failure_form(&OTHER),
            Action::Overwrite,
        );
    });
    let extra = panic_message(|| {
        run(
            &NO_FORM,
            &examples(&dir).failure_form(&FORM),
            Action::Overwrite,
        );
    });
    run(
        &QUIET,
        &examples(&dir).failure_form(&FORM),
        Action::Overwrite,
    );

    assert_eq!(
        none.unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the guide sets the failure form sh/error/v1, and Examples names no failure form, \
         so check_twin cannot read the tool's failures. Examples::failure_form names the guide's form"
    );
    assert_eq!(
        other.unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the guide sets the failure form sh/error/v1, and Examples::failure_form names sh/fault/v1"
    );
    assert_eq!(
        extra.unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         Examples::failure_form names sh/error/v1, and the guide sets no failure form"
    );
}
