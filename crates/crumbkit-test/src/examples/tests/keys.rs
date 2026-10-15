//! Tests for the example check's lines of a `--json` example whose document
//! has keys that the tool declares (ADR-0059).

use crumbkit::help::{Example, Guide, Leaf};
use crumbkit::{Document, Row};
use snapbox::assert::Action;

use super::{examples, run};
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

const WORD: Document<Word> = Document::new("sh", "word", 1, "One word.")
    .with_keys("$type", "entry")
    .with_type("sh/get-word/v1");

#[test]
fn a_json_example_of_a_document_with_keys_passes_when_its_type_key_holds_the_type() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(
            &[
                r#"echo '{"$type":"sh/get-word/v1","entry":{"word":"a"}}'"#,
                "--json",
            ],
            "Print one word",
        )])
        .document(WORD)]);
    let dir = TempDir::new("examples-keys-pass").unwrap();

    run(&GUIDE, &examples(&dir), Action::Overwrite);
}

#[test]
fn a_json_example_of_a_document_with_keys_fails_when_it_prints_crumbkits_schema_key() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(
            &[
                r#"echo '{"schema":"sh/word/v1","data":{"word":"a"}}'"#,
                "--json",
            ],
            "Print one word",
        )])
        .document(WORD)]);
    let dir = TempDir::new("examples-keys-fail").unwrap();

    let message = panic_message(|| run(&GUIDE, &examples(&dir), Action::Overwrite)).unwrap();

    assert!(
        message.ends_with(
            "--json: line 1 of the --json output must be a JSON object whose $type is sh/get-word/v1"
        ),
        "{message}"
    );
}
