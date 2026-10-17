//! Tests for the example check's timeout and its check of the lines that a
//! `--json` example prints.

use std::time::Duration;

use crumbkit::help::{Example, Guide, Leaf};
use crumbkit::{Document, Row, Stream};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{examples, sh};
use crate::examples::check;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crumbkit::Envelope;

/// A row type for the documents of the tests' leaves.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Word {
    /// The word.
    word: String,
}

impl Row for Word {
    const COLUMNS: &'static [&'static str] = &["word"];
}

const WORD: Document<Word> = Document::new("sh", "word", 1, "One word.");
const WORDS: Document<Stream<Word>> = Document::new("sh", "words", 1, "Each word.");

/// A script that prints two lines of `sh/words/v1`.
const TWO_WORDS: &str = r#"echo '{"schema":"sh/words/v1","data":{"word":"a"}}'; echo '{"schema":"sh/words/v1","data":{"word":"b"}}'"#;

/// Runs the check of `guide` on `sh()` with the golden files in `dir`,
/// `action` and a run timeout of `timeout`.
fn run(guide: &Guide, dir: &TempDir, action: Action, timeout: Duration) {
    check(
        &sh(),
        guide,
        &examples(dir),
        &Assert::new().action(action),
        timeout,
    );
}

#[test]
fn a_run_that_does_not_end_within_the_timeout_is_stopped_and_fails_the_check() {
    const SLOW: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["exec sleep 5"], "Wait")])]);
    let dir = TempDir::new("examples-timeout").unwrap();

    let message = panic_message(|| {
        run(&SLOW, &dir, Action::Overwrite, Duration::from_millis(200));
    });

    assert_eq!(
        message.unwrap(),
        "these runs of the tool's examples fail:\n\
         sh -c 'exec sleep 5': the run did not end within 0.2 seconds, so the check stopped it, and an example must end on its own"
    );
}

#[test]
fn a_json_example_of_a_document_that_prints_two_lines_fails_the_check() {
    const DOCUMENT: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(&[TWO_WORDS, "--json"], "Print two lines")])
        .document(WORD)]);
    let dir = TempDir::new("examples-document-lines").unwrap();

    let message = panic_message(|| {
        run(&DOCUMENT, &dir, Action::Overwrite, Duration::from_secs(10));
    });

    let message = message.unwrap();
    assert!(
        message.ends_with(
            "--json: the command prints one document, so its --json output must be exactly one line, and the run printed 2 lines"
        ),
        "{message}"
    );
}

#[test]
fn a_json_example_of_a_stream_may_print_any_number_of_lines() {
    const STREAM: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[
            Example::new(&[TWO_WORDS, "--json"], "Print two lines"),
            Example::new(&["true", "--json"], "Print nothing"),
        ])
        .stream(WORDS)]);
    let dir = TempDir::new("examples-stream-lines").unwrap();

    run(&STREAM, &dir, Action::Overwrite, Duration::from_secs(10));
    run(&STREAM, &dir, Action::Verify, Duration::from_secs(10));
}

#[test]
fn a_json_line_that_is_not_a_document_of_the_leaf_fails_the_check() {
    const STREAM: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(
            &[
                r#"echo '{"schema":"sh/word/v1","data":{}}'; echo plain"#,
                "--json",
            ],
            "Print other lines",
        )])
        .stream(WORDS)]);
    let dir = TempDir::new("examples-stream-schema").unwrap();

    let message = panic_message(|| {
        run(&STREAM, &dir, Action::Overwrite, Duration::from_secs(10));
    });

    let message = message.unwrap();
    assert!(
        message.contains(
            "--json: line 1 of the --json output must be a JSON object whose schema is sh/words/v1"
        ),
        "{message}"
    );
    assert!(
        message.ends_with(
            "--json: line 2 of the --json output must be a JSON object whose schema is sh/words/v1"
        ),
        "{message}"
    );
}

#[test]
fn a_json_example_of_a_document_without_the_envelope_is_not_checked_for_lines() {
    const UNWRAPPED: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(
            &["echo a; echo b", "--json"],
            "Print two lines",
        )])
        .document(WORD.envelope(Envelope::Without))]);
    let dir = TempDir::new("examples-unwrapped-lines").unwrap();

    run(&UNWRAPPED, &dir, Action::Overwrite, Duration::from_secs(10));
}
