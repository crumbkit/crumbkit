//! Tests for how a failure's JSON document holds the related diagnostics of
//! its miette diagnostic.

use std::fmt;

use miette::{Diagnostic, LabeledSpan, NamedSource, Severity, SourceCode};

use super::super::{PreparedFailure, Printing};
use crate::{Code, Failure, Recovery, Spelling};

const BAD_CONFIG: Code = Code::new("shelf", "config", "bad");

/// The settings file that the failure points into. `colour` starts at byte
/// 22 on line 3 and `size` at byte 37 on line 4.
const SHELF_TOML: &str = "[shelf]\nname = \"mine\"\ncolour = \"red\"\nsize = 3\n";

/// An error below a related diagnostic.
#[derive(Debug)]
struct Cause(&'static str);

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Cause {}

/// A related diagnostic whose every part a test chooses.
#[derive(Debug, Default)]
struct Note {
    message: &'static str,
    code: Option<&'static str>,
    severity: Option<Severity>,
    help: Option<&'static str>,
    url: Option<&'static str>,
    cause: Option<Cause>,
    source: Option<&'static str>,
    labels: Vec<LabeledSpan>,
    related: Vec<Note>,
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for Note {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| cause as &(dyn std::error::Error + 'static))
    }
}

impl Diagnostic for Note {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.code
            .map(|code| Box::new(code) as Box<dyn fmt::Display + 'a>)
    }

    fn severity(&self) -> Option<Severity> {
        self.severity
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .map(|help| Box::new(help) as Box<dyn fmt::Display + 'a>)
    }

    fn url<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.url
            .map(|url| Box::new(url) as Box<dyn fmt::Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        self.source.as_ref().map(|source| source as &dyn SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            return None;
        }
        Some(Box::new(self.labels.iter().cloned()))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        if self.related.is_empty() {
            return None;
        }
        Some(Box::new(
            self.related.iter().map(|note| note as &dyn Diagnostic),
        ))
    }
}

/// A failure that points into `SHELF_TOML` and has `related` as its related
/// diagnostics.
#[derive(Debug)]
struct BadConfig {
    source: NamedSource<&'static str>,
    related: Vec<Note>,
}

impl fmt::Display for BadConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the settings are not valid")
    }
}

impl std::error::Error for BadConfig {}

impl Diagnostic for BadConfig {
    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source)
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(
            self.related.iter().map(|note| note as &dyn Diagnostic),
        ))
    }
}

impl Failure for BadConfig {
    fn code(&self) -> &'static Code {
        &BAD_CONFIG
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("fix the settings".to_owned())
    }

    fn diagnostic(&self) -> Option<&dyn Diagnostic> {
        Some(self)
    }
}

/// Returns the JSON failure document of a failure with `related`.
fn json(related: Vec<Note>) -> serde_json::Value {
    let failure = BadConfig {
        source: NamedSource::new("shelf.toml", SHELF_TOML),
        related,
    };
    serde_json::to_value(PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    ))
    .unwrap()
}

#[test]
fn json_holds_each_related_diagnostic_with_every_part_it_has_after_the_labels() {
    let note = Note {
        message: "`colour` is not a key",
        code: Some("toml::unknown_key"),
        severity: Some(Severity::Warning),
        help: Some("use `color`"),
        url: Some("https://example.com/keys"),
        cause: Some(Cause("the schema has no such key")),
        labels: vec![LabeledSpan::new(Some("here".to_owned()), 22, 6)],
        ..Note::default()
    };

    let failure = BadConfig {
        source: NamedSource::new("shelf.toml", SHELF_TOML),
        related: vec![note],
    };
    let document = PreparedFailure::new(
        &failure,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    assert_eq!(
        serde_json::to_string(&document).unwrap(),
        concat!(
            r#"{"schema":"crumbkit/failure/v1","code":"shelf::config::bad","message":"the settings are not valid","#,
            r#""related":[{"message":"`colour` is not a key","code":"toml::unknown_key","severity":"warning","#,
            r#""cause":"the schema has no such key","#,
            r#""labels":[{"offset":22,"length":6,"name":"shelf.toml","line":3,"column":1,"excerpt":{"line":2,"text":"name = \"mine\"\ncolour = \"red\"\nsize = 3"},"text":"here"}],"#,
            r#""help":"use `color`","url":"https://example.com/keys"}],"#,
            r#""recovery":{"kind":"hint","text":"fix the settings"}}"#
        )
    );
}

#[test]
fn a_related_diagnostic_without_parts_holds_its_message_and_the_error_severity() {
    let json = json(vec![Note {
        message: "see also",
        ..Note::default()
    }]);

    assert_eq!(
        json["related"],
        serde_json::json!([{ "message": "see also", "severity": "error" }])
    );
}

#[test]
fn a_related_diagnostic_reads_its_labels_against_its_own_source_or_else_the_failures() {
    let json = json(vec![
        Note {
            message: "own source",
            source: Some("size = 3\n"),
            labels: vec![LabeledSpan::new(None, 0, 4)],
            ..Note::default()
        },
        Note {
            message: "the failure's source",
            labels: vec![LabeledSpan::new(None, 37, 4)],
            ..Note::default()
        },
    ]);

    assert_eq!(json["related"][0]["labels"][0]["line"], 1);
    assert_eq!(json["related"][1]["labels"][0]["name"], "shelf.toml");
    assert_eq!(json["related"][1]["labels"][0]["line"], 4);
}

#[test]
fn the_related_diagnostics_of_a_related_diagnostic_nest_under_it() {
    let json = json(vec![Note {
        message: "outer",
        related: vec![Note {
            message: "inner",
            ..Note::default()
        }],
        ..Note::default()
    }]);

    assert_eq!(json["related"][0]["related"][0]["message"], "inner");
}

/// A failure whose diagnostic lists itself as related.
#[derive(Debug)]
struct Loop;

impl fmt::Display for Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("loop")
    }
}

impl std::error::Error for Loop {}

impl Diagnostic for Loop {
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(std::iter::once(self as &dyn Diagnostic)))
    }
}

impl Failure for Loop {
    fn code(&self) -> &'static Code {
        &BAD_CONFIG
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }

    fn diagnostic(&self) -> Option<&dyn Diagnostic> {
        Some(self)
    }
}

#[test]
fn related_diagnostics_nest_at_most_eight_levels_deep() {
    let document = PreparedFailure::new(
        &Loop,
        Spelling::COLONS,
        Printing::default(),
        &mut clap::Command::new("tool"),
        &mut std::io::sink(),
    );

    let json = serde_json::to_string(&document).unwrap();

    assert_eq!(json.matches(r#""related":"#).count(), 8, "{json}");
}
