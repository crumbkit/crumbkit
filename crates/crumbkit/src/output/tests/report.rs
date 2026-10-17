//! Tests for converting a `miette::Report` into an `Error`.

use std::fmt;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode, miette};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{Document, Error, Output, Row, Sink, Spelling, finish_to};

/// Joins the segments of a code with `.`, so a test shows that the code
/// prints in the spelling that the tool passes and not in the colon
/// spelling.
const DOTS: Spelling =
    Spelling::custom(|code, f| write!(f, "{}.{}.{}", code.component(), code.area(), code.reason()));

/// A row type for the result that the failing command would have printed.
#[derive(Serialize, JsonSchema)]
struct Setting {
    key: &'static str,
}

impl Row for Setting {
    const COLUMNS: &'static [&'static str] = &["key"];
}

const SETTINGS: Document<Vec<Setting>> = Document::new("shelf", "settings", 1, "The test rows.");

/// Runs `parse` in a handler that returns `crumbkit::Error`, so `?` converts
/// the report, and returns the failure's text form.
fn printed(parse: impl FnOnce() -> miette::Result<Vec<Setting>>) -> String {
    let handler = || -> Result<Vec<Setting>, Error> {
        let settings = parse()?;
        Ok(settings)
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut output = Output::of(clap::Command::new("shelf"));
    finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        DOTS,
        SETTINGS,
        handler(),
    );
    String::from_utf8(err).unwrap()
}

#[test]
fn a_report_with_a_code_help_and_url_prints_its_code_in_the_message_and_its_help_as_a_hint() {
    let text = printed(|| {
        Err(miette!(
            code = "shelf::config::unknown_key",
            help = "use `color`",
            url = "https://example.com/keys",
            "unknown key"
        ))
    });

    let expected = "\
error: unknown key (shelf::config::unknown_key)
  help:  use `color`
  code:  shelf.report.unchecked
  url:   https://example.com/keys
";
    assert_eq!(text, expected);
}

#[test]
fn a_report_without_a_code_or_help_prints_its_causes_and_no_next_step() {
    let text =
        printed(|| Err(miette!("permission denied").wrap_err("the settings cannot be read")));

    let expected = "\
error: the settings cannot be read
  cause: permission denied
  help:  no next step: the error gives no advice
  code:  shelf.report.unchecked
";
    assert_eq!(text, expected);
}

/// A diagnostic that points at an unknown key in a settings file.
#[derive(Debug)]
struct UnknownKey {
    source: NamedSource<&'static str>,
}

impl fmt::Display for UnknownKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown key `colour`")
    }
}

impl std::error::Error for UnknownKey {}

impl Diagnostic for UnknownKey {
    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let label = LabeledSpan::new(Some("this key is not known".to_owned()), 8, 6);
        Some(Box::new(std::iter::once(label)))
    }
}

#[test]
fn a_report_prints_the_labels_of_its_diagnostic() {
    let text = printed(|| {
        Err(miette::Report::new(UnknownKey {
            source: NamedSource::new("shelf.toml", "[shelf]\ncolour = \"red\"\n"),
        }))
    });

    let expected = "\
error: unknown key `colour`
  at:    shelf.toml:2:1: this key is not known
     1 | [shelf]
     2 | colour = \"red\"
       | ^^^^^^
  help:  no next step: the error gives no advice
  code:  shelf.report.unchecked
";
    assert_eq!(text, expected);
}
