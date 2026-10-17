//! Tests for how `finish_to` lays out a JSON document when the guide sets
//! `JsonLayout::Pretty` (ADR-0052).

use std::fmt;
use std::process::ExitCode;

use clap::CommandFactory as _;

use super::{Finished, PLANET, PLANETS, Tool, finish_output, saturn};
use crate::Envelope;
use crate::help::Guide;
use crate::{
    Call, Code, Document, Error, Failure, JsonStyle, Output, Recovery, Rows, Sink, Spelling,
    finish_to,
};

/// The guide of a tool whose readers expect indented JSON.
const PRETTY: Guide = Guide::new().json(JsonStyle::new().pretty("the readers expect it"));

/// Returns the output flags for `tool list` followed by `flags`, with the
/// guide [`PRETTY`], as `Output::new` builds them.
fn pretty(flags: &[&str]) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    super::built_with(Tool::command(), &PRETTY, &args)
}

/// Finishes `value` with `flags` and the guide [`PRETTY`].
fn finish_pretty<T: Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    finish_output(&mut pretty(flags), document, value)
}

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A failure whose recovery names `tool list`.
#[derive(Debug)]
struct Missing;

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the solar system has no planet Vulcan")
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }
}

#[test]
fn a_pretty_layout_indents_the_envelope_by_two_spaces() {
    let finished = finish_pretty(&["--json"], PLANETS, vec![saturn()]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    let expected = r#"{
  "schema": "solar/planets/v1",
  "data": [
    {
      "name": "Saturn",
      "moons": 274
    }
  ]
}
"#;
    assert_eq!(finished.out, expected);
}

#[test]
fn a_pretty_layout_indents_a_document_without_the_envelope() {
    let finished = finish_pretty(&["--json"], PLANET.envelope(Envelope::Without), saturn());

    let expected = r#"{
  "name": "Saturn",
  "moons": 274,
  "ringed": true,
  "au": 9.54
}
"#;
    assert_eq!(finished.out, expected);
}

#[test]
fn a_pretty_layout_leaves_the_text_table_unchanged() {
    let finished = finish_pretty(&[], PLANETS, vec![saturn()]);

    assert_eq!(finished.out, "NAME    MOONS\nSaturn  274\n");
}

#[test]
fn a_pretty_layout_indents_the_failure_document() {
    let mut output = pretty(&["--json"]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let result: Result<Vec<super::Planet>, Error> = Err(Error::from(Missing));

    let code = finish_to(&mut sink, &mut output, Spelling::COLONS, PLANETS, result);

    assert_eq!(code, 1);
    let out = String::from_utf8(out).unwrap();
    let expected = r#"{
  "schema": "tool/failure/v1",
  "code": "solar::planets::not_found",
  "message": "the solar system has no planet Vulcan",
  "recovery": {
    "kind": "run",
    "call": "tool list"
  }
}
"#;
    assert_eq!(out, expected);
}
