//! Tests for how a failure prints the labels of its miette diagnostic.

use std::fmt;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};

use super::super::{Entry, FailureDocument, write_text};
use crate::{Code, Failure, Recovery, Spelling};

const UNKNOWN_KEY: Code = Code::new("shelf", "config", "unknown_key");

/// The settings file that most tests point into. `colour` starts at byte 22
/// on line 3.
const SHELF_TOML: &str = "[shelf]\nname = \"mine\"\ncolour = \"red\"\nsize = 3\n";

/// A failure that points into `source` with the labels a test chooses. It
/// implements `Diagnostic` by hand and returns itself from `diagnostic()`.
#[derive(Debug)]
struct UnknownKey<S> {
    source: S,
    labels: Vec<LabeledSpan>,
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
fn in_shelf_toml(labels: Vec<LabeledSpan>) -> UnknownKey<NamedSource<&'static str>> {
    UnknownKey {
        source: NamedSource::new("shelf.toml", SHELF_TOML),
        labels,
    }
}

/// Returns the label on `colour` on line 3 of `SHELF_TOML`.
fn on_colour() -> LabeledSpan {
    LabeledSpan::new(Some("this key is not known".to_owned()), 22, 6)
}

/// Returns the text form of `failure`.
fn text(failure: &dyn Failure) -> String {
    let document = FailureDocument::new(failure, Spelling::COLONS, None, &mut std::io::sink());
    let mut err = Vec::new();
    write_text(&mut err, &document).unwrap();
    String::from_utf8(err).unwrap()
}

#[test]
fn a_label_on_a_named_source_prints_an_at_line_then_the_source_line_and_a_caret_underline() {
    let failure = in_shelf_toml(vec![on_colour()]);

    let expected = "\
error: unknown key `colour`
  at:    shelf.toml:3:1: this key is not known
     3 | colour = \"red\"
       | ^^^^^^
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_label_on_an_unnamed_source_prints_its_line_and_column_without_a_name() {
    let failure = UnknownKey {
        source: "colour = \"red\"\n",
        labels: vec![LabeledSpan::new(Some("not a colour".to_owned()), 9, 5)],
    };

    let expected = "\
error: unknown key `colour`
  at:    1:10: not a colour
     1 | colour = \"red\"
       |          ^^^^^
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn two_labels_print_in_the_order_the_diagnostic_gives_them_after_the_causes() {
    let failure = in_shelf_toml(vec![
        on_colour(),
        LabeledSpan::new(Some("the section starts here".to_owned()), 0, 7),
    ]);

    let expected = "\
error: unknown key `colour`
  at:    shelf.toml:3:1: this key is not known
     3 | colour = \"red\"
       | ^^^^^^
  at:    shelf.toml:1:1: the section starts here
     1 | [shelf]
       | ^^^^^^^
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_label_without_text_prints_only_its_place() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 22, 6)]);

    assert!(
        text(&failure).contains("\n  at:    shelf.toml:3:1\n     3 | colour"),
        "{}",
        text(&failure)
    );
}

#[test]
fn a_span_that_runs_past_its_line_is_underlined_to_the_end_of_that_line() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 31, 10)]);

    let expected = "  at:    shelf.toml:3:10
     3 | colour = \"red\"
       |          ^^^^^
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn an_empty_span_is_underlined_with_one_caret() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 28, 0)]);

    let expected = "  at:    shelf.toml:3:7
     3 | colour = \"red\"
       |       ^
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn a_span_that_cannot_be_read_prints_its_byte_offset_and_no_source_line() {
    let failure = in_shelf_toml(vec![LabeledSpan::new(
        Some("this key is not known".to_owned()),
        400,
        6,
    )]);

    let expected = "\
error: unknown key `colour`
  at:    byte 400: this key is not known
  help:  use `color`
  code:  shelf::config::unknown_key
";
    assert_eq!(text(&failure), expected);
}

#[test]
fn a_control_character_in_the_source_line_or_the_label_prints_as_its_escape() {
    let failure = UnknownKey {
        source: "\tcolour = \"red\"\r\n",
        labels: vec![LabeledSpan::new(Some("not\nknown".to_owned()), 1, 6)],
    };

    let expected = "  at:    1:2: not\\nknown
     1 | \\tcolour = \"red\"
       |   ^^^^^^
";
    assert!(text(&failure).contains(expected), "{}", text(&failure));
}

#[test]
fn json_holds_each_label_with_its_place_when_the_span_could_be_read() {
    let failure = in_shelf_toml(vec![on_colour(), LabeledSpan::new(None, 400, 6)]);
    let document = FailureDocument::new(&failure, Spelling::COLONS, None, &mut std::io::sink());

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
    let document = FailureDocument::new(&failure, Spelling::COLONS, None, &mut std::io::sink());

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
    let document = FailureDocument::new(&failure, Spelling::COLONS, None, &mut std::io::sink());

    let json = serde_json::to_string(&document).unwrap();

    assert!(!json.contains("labels"), "{json}");
}

#[test]
fn a_label_excerpt_holds_every_line_of_its_span_and_one_line_before_and_after() {
    // The span runs from `name` on line 2 into `colour` on line 3.
    let failure = in_shelf_toml(vec![LabeledSpan::new(None, 8, 20)]);
    let document = FailureDocument::new(&failure, Spelling::COLONS, None, &mut std::io::sink());

    let json = serde_json::to_value(&document).unwrap();

    assert_eq!(
        json["labels"][0]["excerpt"],
        serde_json::json!({
            "line": 1,
            "text": "[shelf]\nname = \"mine\"\ncolour = \"red\"\nsize = 3",
        })
    );
}
