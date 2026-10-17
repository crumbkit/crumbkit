//! Tests for how a failure reads the labels of its miette diagnostic and
//! holds them in JSON. The tests of the text form are in `text/tests.rs`,
//! which shares the failure that these tests build.

use std::fmt;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

use super::super::{Entry, PreparedFailure, Printing};
use crate::{Code, Failure, Recovery, Spelling};

const UNKNOWN_KEY: Code = Code::new("shelf", "config", "unknown_key");

/// The settings file that most tests point into. `colour` starts at byte 22
/// on line 3.
pub(super) const SHELF_TOML: &str = "[shelf]\nname = \"mine\"\ncolour = \"red\"\nsize = 3\n";

/// A failure that points into `source` with the labels a test chooses. It
/// implements `Diagnostic` by hand and returns itself from `diagnostic()`.
#[derive(Debug)]
pub(super) struct UnknownKey<S> {
    pub(super) source: S,
    pub(super) labels: Vec<LabeledSpan>,
}

impl<S> fmt::Display for UnknownKey<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown key `colour`")
    }
}

impl<S: fmt::Debug> std::error::Error for UnknownKey<S> {}

impl<S: SourceCode + fmt::Debug> Diagnostic for UnknownKey<S> {
    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(self.labels.iter().cloned()))
    }
}

impl<S: SourceCode + fmt::Debug> Failure for UnknownKey<S> {
    fn code(&self) -> &'static Code {
        &UNKNOWN_KEY
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("use `color`".to_owned())
    }

    fn diagnostic(&self) -> Option<&dyn Diagnostic> {
        Some(self)
    }
}

/// Returns a failure that points into `SHELF_TOML`, named `shelf.toml`, with
/// `labels`.
pub(super) fn in_shelf_toml(labels: Vec<LabeledSpan>) -> UnknownKey<NamedSource<&'static str>> {
    UnknownKey {
        source: NamedSource::new("shelf.toml", SHELF_TOML),
        labels,
    }
}

/// Returns the label on `colour` on line 3 of `SHELF_TOML`.
pub(super) fn on_colour() -> LabeledSpan {
    LabeledSpan::new(Some("this key is not known".to_owned()), 22, 6)
}

#[test]
fn json_holds_each_label_with_its_place_when_the_span_could_be_read() {
    let failure = in_shelf_toml(vec![on_colour(), LabeledSpan::new(None, 400, 6)]);
    let document = PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    let json = serde_json::to_string(&document).unwrap();

    assert_eq!(
        json,
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"shelf::config::unknown_key","message":"unknown key `colour`","#,
            r#""labels":[{"offset":22,"length":6,"name":"shelf.toml","line":3,"column":1,"excerpt":{"line":2,"text":"name = \"mine\"\ncolour = \"red\"\nsize = 3"},"text":"this key is not known"},{"offset":400,"length":6}],"#,
            r#""recovery":{"kind":"hint","text":"use `color`"}}"#
        )
    );
}

#[test]
fn a_batch_entry_holds_the_labels_as_the_failure_document_does() {
    let failure = UnknownKey {
        source: "colour = \"red\"\n",
        labels: vec![LabeledSpan::new(None, 0, 6)],
    };
    let document = PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    let json = serde_json::to_string(&Entry(&document)).unwrap();

    assert_eq!(
        json,
        concat!(
            r#"{"code":"shelf::config::unknown_key","message":"unknown key `colour`","#,
            r#""labels":[{"offset":0,"length":6,"line":1,"column":1,"excerpt":{"line":1,"text":"colour = \"red\""}}],"#,
            r#""recovery":{"kind":"hint","text":"use `color`"}}"#
        )
    );
}

#[test]
fn json_leaves_out_labels_when_the_diagnostic_has_none() {
    let failure = in_shelf_toml(Vec::new());
    let document = PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    let json = serde_json::to_string(&document).unwrap();

    assert!(!json.contains("labels"), "{json}");
}

#[test]
fn a_label_excerpt_holds_every_line_of_its_span_and_one_line_before_and_after() {
    // The span runs from `name` on line 2 into `colour` on line 3.
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 8, 20)]);
    let document = PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    let json = serde_json::to_value(&document).unwrap();

    assert_eq!(
        json["labels"][0]["excerpt"],
        serde_json::json!({
            "line": 1,
            "text": "[shelf]\nname = \"mine\"\ncolour = \"red\"\nsize = 3",
        })
    );
}
