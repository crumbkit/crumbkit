//! Tests for the example check's lines of a `--json` example when the tool
//! record sets `JsonLayout::Pretty`.

use std::time::Duration;

use crumbkit::help::{Example, Help, Leaf};
use crumbkit::{Document, JsonLayout, Row, Stream, Tool};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{examples, sh};
use crate::examples::check;
use crate::temp::TempDir;
use crate::testing::panic_message;

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

/// A script that prints one `sh/word/v1` document indented by two spaces.
const INDENTED: &str =
    r#"printf '{\n  "schema": "sh/word/v1",\n  "data": {\n    "word": "a"\n  }\n}\n'"#;

/// A script that prints one `sh/word/v1` document on one line.
const ONE_LINE: &str = r#"echo '{"schema":"sh/word/v1","data":{"word":"a"}}'"#;

/// A script that prints two lines of `sh/words/v1`.
const TWO_WORDS: &str = r#"echo '{"schema":"sh/words/v1","data":{"word":"a"}}'; echo '{"schema":"sh/words/v1","data":{"word":"b"}}'"#;

/// Runs the check of `tool`, with the layout `JsonLayout::Pretty`, on
/// `sh()` with the golden files in `dir` and the action `Overwrite`.
fn run(tool: &Tool, dir: &TempDir) {
    let tool = tool.output(tool.get_output().json_layout(JsonLayout::Pretty {
        reason: "the readers expect it",
    }));
    check(
        &sh(),
        &tool,
        &examples(dir),
        &Assert::new().action(Action::Overwrite),
        Duration::from_secs(10),
    );
}

#[test]
fn a_pretty_layout_lets_a_json_example_of_a_document_print_it_indented() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(&[INDENTED, "--json"], "Print one word")])
            .document(WORD)]),
    );
    let dir = TempDir::new("examples-pretty-indented").unwrap();

    run(&TOOL, &dir);
}

#[test]
fn a_pretty_layout_fails_a_json_example_that_prints_its_document_on_one_line() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(&[ONE_LINE, "--json"], "Print one word")])
            .document(WORD)]),
    );
    let dir = TempDir::new("examples-pretty-one-line").unwrap();

    let message = panic_message(|| run(&TOOL, &dir)).unwrap();

    assert!(
        message.ends_with(
            "--json: the tool record sets JsonLayout::Pretty, so the --json output must be one document indented by two spaces, and the run printed it on one line"
        ),
        "{message}"
    );
}

#[test]
fn a_pretty_layout_fails_a_json_example_that_prints_two_documents() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(&[TWO_WORDS, "--json"], "Print two lines")])
            .document(WORD)]),
    );
    let dir = TempDir::new("examples-pretty-two").unwrap();

    let message = panic_message(|| run(&TOOL, &dir)).unwrap();

    assert!(
        message.ends_with(
            "--json: the command prints one document, so its --json output must be exactly one JSON object whose schema is sh/word/v1, and the run printed 2 JSON documents"
        ),
        "{message}"
    );
}

#[test]
fn a_pretty_layout_keeps_one_line_for_each_document_of_a_stream() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(&[TWO_WORDS, "--json"], "Print two lines")])
            .stream(WORDS)]),
    );
    let dir = TempDir::new("examples-pretty-stream").unwrap();

    run(&TOOL, &dir);
}
