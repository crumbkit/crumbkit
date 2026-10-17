//! Tests for how `finish_to` orders the keys of a JSON document when the
//! guide sets `JsonStyle::sorted` (ADR-0067).

use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};
use schemars::JsonSchema;
use serde::Serialize;

use super::{Finished, PLANET, PLANETS, Planet, Tool, finish_output, mercury, saturn};
use crate::Envelope;
use crate::help::Guide;
use crate::{
    Batch, Bytes, Document, Error, Headed, JsonStyle, Listing, Output, Parsed, Row, Sink, Spelling,
    finish_to,
};

/// The guide of a tool whose readers expect sorted keys.
const SORTED: Guide = Guide::new().json(JsonStyle::new().sorted());

/// Returns the output flags for `tool list` followed by `flags`, with
/// `guide`, as a tool's `main` sets them.
fn guided(flags: &[&str], guide: &'static Guide) -> Output {
    let mut command = Tool::command();
    let args = ["tool", "list"].iter().chain(flags);
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let mut output = Output::from_arg_matches(&matches).unwrap();
    output.set_command(Parsed::guided(command, guide));
    output
}

/// Finishes `value` with `flags` and the guide [`SORTED`].
fn finish_sorted<T: crate::Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    finish_output(&mut guided(flags, &SORTED), document, value)
}

#[test]
fn sorted_keys_order_the_envelope_and_every_field_of_each_row_by_name() {
    let finished = finish_sorted(&["--json", "--full"], PLANETS, vec![saturn()]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"data":[{"au":9.54,"moons":274,"name":"Saturn","ringed":true}],"#,
            r#""schema":"solar/planets/v1"}"#,
            "\n"
        )
    );
}

#[test]
fn sorted_keys_order_the_declared_keys_among_crumbkits_own_names() {
    let document: Document<Listing<Planet>> =
        Document::new("solar", "planet-listing", 1, "The test rows.").envelope(Envelope::Keys {
            type_key: "type",
            data_key: "entries",
        });

    let finished = finish_sorted(
        &["--json"],
        document,
        Listing::new(vec![mercury(), saturn()], 8),
    );

    assert_eq!(
        finished.out,
        concat!(
            r#"{"entries":[{"moons":0,"name":"Mercury"},{"moons":274,"name":"Saturn"}],"#,
            r#""total":8,"truncated":true,"type":"solar/planet-listing/v1"}"#,
            "\n"
        )
    );
}

#[test]
fn sorted_keys_order_a_document_without_the_envelope() {
    let finished = finish_sorted(&["--json"], PLANET.envelope(Envelope::Without), saturn());

    assert_eq!(
        finished.out,
        "{\"au\":9.54,\"moons\":274,\"name\":\"Saturn\",\"ringed\":true}\n"
    );
}

#[test]
fn sorted_keys_with_a_pretty_layout_indent_the_sorted_document() {
    const PRETTY_SORTED: Guide =
        Guide::new().json(JsonStyle::new().sorted().pretty("the readers expect it"));

    let finished = finish_output(
        &mut guided(&["--json"], &PRETTY_SORTED),
        PLANET.envelope(Envelope::Without),
        saturn(),
    );

    let expected = r#"{
  "au": 9.54,
  "moons": 274,
  "name": "Saturn",
  "ringed": true
}
"#;
    assert_eq!(finished.out, expected);
}

#[test]
fn sorted_keys_order_the_failure_document_and_its_recovery() {
    let mut output = guided(&["--json"], &SORTED);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<Planet>, Error> = Err(Error::unclassified("the catalogue is missing"));

    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        PLANETS,
        result,
    );

    assert_eq!(code, 1);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            r#"{"code":"crumbkit::report::unchecked","message":"the catalogue is missing","#,
            r#""recovery":{"kind":"none","reason":"the error gives no advice"},"#,
            r#""schema":"crumbkit/failure/v1"}"#,
            "\n"
        )
    );
}

/// A record that a headed batch prints as one line of JSON, because it
/// holds no bytes.
#[derive(Serialize, JsonSchema)]
struct Record {
    /// The record's title.
    title: &'static str,
    /// The record's author.
    author: &'static str,
}

impl Row for Record {
    const COLUMNS: &'static [&'static str] = &["title"];
}

impl Headed for Record {
    fn bytes(&self) -> Option<&Bytes> {
        None
    }
}

#[test]
fn sorted_keys_order_the_json_line_of_a_headed_answer_without_bytes() {
    let document: Document<Batch<Record, Error>> =
        Document::new("solar", "records", 1, "The records.").headed();
    let mut batch = Batch::new(&["show"], "names", &["a"]);
    batch.push(Ok(Record {
        title: "Sidereus Nuncius",
        author: "Galileo",
    }));

    let finished = finish_sorted(&[], document, batch);

    assert_eq!(
        finished.out,
        "{\"author\":\"Galileo\",\"title\":\"Sidereus Nuncius\"}\n"
    );
}
