//! Tests for a tool's own failure form.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::{JsonSchema, Schema, json_schema};
use serde::Serialize;
use serde_json::{Value, json};

use super::{FailureForm, PreparedFailure, ReadFailure, Step};
use crate::answered::answered;
use crate::help::Guide;
use crate::{
    Batch, Call, Code, Document, Failure, Field, FieldValue, JsonStyle, Recovery, Row, Sink,
    Spelling, Stream, finish_to, parse_to, stream_to,
};

mod entries;
mod keys;

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A row type for the commands of the test tool.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const PLANETS: Document<Vec<Planet>> = Document::new("solar", "planets", 1, "The test rows.");

const PLANETS_SHOW: Document<Batch<Planet, Missing>> =
    Document::new("solar", "planets-show", 1, "The test batch.");

const PLANETS_STREAM: Document<Stream<Planet>> =
    Document::new("solar", "planets-stream", 1, "The test stream.");

/// A tool with a `list` command that a recovery can name.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: crate::ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// A failure with a cause, a recovery that names `tool list`, an
/// alternative, a field and a url.
#[derive(Debug)]
struct Missing;

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the solar system has no planet Vulcan")
    }
}

impl std::error::Error for Missing {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&CAUSE)
    }
}

/// The cause of every `Missing`.
static CAUSE: Cause = Cause;

/// The error below a `Missing`.
#[derive(Debug)]
struct Cause;

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the catalogue is empty")
    }
}

impl std::error::Error for Cause {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }

    fn alternatives(&self) -> Vec<Recovery> {
        vec![Recovery::Hint("ask for Pluto instead".to_owned())]
    }

    fn fields(&self) -> Vec<Field<'_>> {
        vec![Field::new("planet", "Vulcan")]
    }

    fn url(&self) -> Option<&str> {
        Some("https://example.dev/vulcan")
    }
}

/// Returns the text of `step`, as the forms below print it.
fn step(step: &Step) -> String {
    match step {
        Step::Run(call) => format!("run {call}"),
        Step::RunAll { call, field } => format!("run {call} with every {field}"),
        Step::Retry => "retry".to_owned(),
        Step::Hint(text) => format!("hint {text}"),
        Step::None(reason) => format!("none {reason}"),
    }
}

/// A form that prints every prepared value in an `{"error": ...}` document
/// and keeps crumbkit's text.
struct ErrorForm;

impl FailureForm for ErrorForm {
    fn document(&self) -> Document<()> {
        Document::new("solar", "error", 2, "The document of a failure.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object", "required": ["error"] })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        let fields: serde_json::Map<String, Value> = failure
            .fields()
            .map(|(name, value)| (name.to_owned(), serde_json::to_value(value).unwrap()))
            .collect();
        json!({ "error": {
            "code": failure.code(),
            "message": failure.message(),
            "causes": failure.causes(),
            "recovery": step(failure.recovery()),
            "alternatives": failure.alternatives().iter().map(step).collect::<Vec<_>>(),
            "fields": fields,
            "url": failure.url(),
            "display": failure.failure().to_string(),
        }})
    }
}

/// A form with a one-line text of its own.
struct LineForm;

impl FailureForm for LineForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        ErrorForm.json(failure)
    }

    fn write_text(&self, err: &mut dyn Write, failure: &PreparedFailure<'_>) -> io::Result<()> {
        writeln!(err, "{}: {}", failure.code(), failure.message())
    }

    /// Reads the code from the last line of the text, which names no step,
    /// so the failure offers a retry.
    fn read(&self, _out: &str, err: &str) -> Option<ReadFailure> {
        let (code, _) = err.lines().last()?.split_once(": ")?;
        Some(ReadFailure::new(code.to_owned(), vec![Step::Retry]))
    }
}

const ERROR_GUIDE: Guide = Guide::new().failure_form(&ErrorForm);

const LINE_GUIDE: Guide = Guide::new().failure_form(&LineForm);

const PRETTY_GUIDE: Guide = Guide::new()
    .failure_form(&ErrorForm)
    .json(JsonStyle::new().pretty("the readers expect it"));

/// The document that `ErrorForm` prints for a `Missing` spelled with dots.
const ERROR_DOCUMENT: &str = concat!(
    r#"{"error":{"alternatives":["hint ask for Pluto instead"],"causes":["the catalogue is empty"],"code":"solar.planets.not_found","display":"the solar system has no planet Vulcan","fields":{"planet":"Vulcan"},"message":"the solar system has no planet Vulcan","recovery":"run tool list","url":"https://example.dev/vulcan"}}"#,
    "\n"
);

/// What a run printed and returned.
struct Ran {
    code: ExitCode,
    out: String,
    err: String,
}

/// Parses `args` after the program name with `guide`, as a tool's `main`
/// does, and finishes `result` with the parsed output.
fn finish<T: crate::Rows>(
    guide: &Guide,
    args: &[&str],
    document: Document<T>,
    result: Result<T, Missing>,
) -> Ran {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let words = std::iter::once("tool").chain(args.iter().copied());
    let code = match parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        guide,
        Spelling::DOTS,
        words,
    ) {
        Ok((_, mut output)) => ExitCode::from(finish_to(
            &mut sink,
            &mut output,
            Spelling::DOTS,
            document,
            result,
        )),
        Err(stop) => stop.code(),
    };
    Ran {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Finishes the failure `Missing` of `tool list` with `args` and `guide`.
fn fail(guide: &Guide, args: &[&str]) -> Ran {
    let mut words = vec!["list"];
    words.extend(args);
    finish(guide, &words, PLANETS, Err(Missing))
}

#[test]
fn a_failed_command_prints_the_form_document_with_every_prepared_value_on_one_line() {
    let ran = fail(&ERROR_GUIDE, &["--json"]);

    assert_eq!(ran.code, ExitCode::FAILURE);
    assert_eq!(ran.out, ERROR_DOCUMENT);
    assert_eq!(ran.err, "");
}

#[test]
fn a_form_that_writes_only_its_document_keeps_crumbkit_text() {
    let with_form = fail(&ERROR_GUIDE, &[]);
    let without = fail(&Guide::new(), &[]);

    assert_eq!(with_form.code, ExitCode::FAILURE);
    assert_eq!(with_form.out, "");
    assert_eq!(with_form.err, without.err);
    assert!(
        without
            .err
            .starts_with("error: the solar system has no planet Vulcan\n")
    );
}

#[test]
fn a_form_with_its_own_text_prints_it_on_standard_error() {
    let ran = fail(&LINE_GUIDE, &[]);

    assert_eq!(ran.code, ExitCode::FAILURE);
    assert_eq!(ran.out, "");
    assert_eq!(
        ran.err,
        "solar.planets.not_found: the solar system has no planet Vulcan\n"
    );
}

#[test]
fn a_command_line_that_clap_refuses_prints_in_the_form_and_exits_with_code_2() {
    let ran = finish(&LINE_GUIDE, &["lst", "--json"], PLANETS, Ok(Vec::new()));
    let text = finish(&LINE_GUIDE, &["lst"], PLANETS, Ok(Vec::new()));

    assert_eq!(ran.code, ExitCode::from(2));
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(document["error"]["code"], "crumbkit.cli.usage");
    assert_eq!(document["error"]["fields"], json!({ "subcommand": "lst" }));
    assert_eq!(text.code, ExitCode::from(2));
    assert!(
        text.err
            .starts_with("crumbkit.cli.usage: unrecognized subcommand"),
        "{}",
        text.err
    );
}

#[test]
fn a_fields_name_that_finish_refuses_prints_in_the_form() {
    let ran = finish(
        &ERROR_GUIDE,
        &["list", "--json", "--fields", "radius"],
        PLANETS,
        Ok(Vec::new()),
    );

    assert_eq!(ran.code, ExitCode::from(2));
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(document["error"]["code"], "crumbkit.cli.usage");
    assert_eq!(
        document["error"]["fields"],
        json!({ "argument": "--fields", "value": "radius" })
    );
}

#[test]
fn a_batch_prints_its_failures_in_the_form_text_and_a_form_without_its_own_entry_keeps_crumbkit_entries()
 {
    let batch = || answered(vec![Planet { name: "Mars" }], vec![Missing]);
    let text = finish(&LINE_GUIDE, &["list"], PLANETS_SHOW, Ok(batch()));
    let json = finish(&LINE_GUIDE, &["list", "--json"], PLANETS_SHOW, Ok(batch()));

    assert_eq!(text.code, ExitCode::FAILURE);
    assert_eq!(text.out, "NAME\nMars\n");
    assert_eq!(
        text.err,
        "solar.planets.not_found: the solar system has no planet Vulcan\n"
    );
    let document: Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(document["failures"][0]["code"], "solar.planets.not_found");
    assert_eq!(
        document["failures"][0]["recovery"],
        json!({ "kind": "run", "call": "tool list" })
    );
}

#[test]
fn a_failure_in_a_json_stream_prints_one_line_of_the_form_document_between_the_rows() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let (_, mut output) = parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        &ERROR_GUIDE,
        Spelling::DOTS,
        ["tool", "list", "--json"],
    )
    .unwrap();
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::DOTS, PLANETS_STREAM).unwrap();
    let _ = streaming.item(&Planet { name: "Mars" });
    let _ = streaming.failure(&Missing);
    let _ = streaming.item(&Planet { name: "Venus" });
    let code = streaming.end(Ok::<(), Missing>(()));

    assert_eq!(code, ExitCode::FAILURE);
    let out = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(format!("{}\n", lines[1]), ERROR_DOCUMENT);
    assert!(lines[2].contains("Venus"));
}

#[test]
fn a_pretty_layout_indents_the_form_document() {
    let ran = fail(&PRETTY_GUIDE, &["--json"]);

    assert_eq!(ran.code, ExitCode::FAILURE);
    let document: Value = serde_json::from_str(ERROR_DOCUMENT).unwrap();
    let indented = serde_json::to_string_pretty(&document).unwrap() + "\n";
    assert_eq!(ran.out, indented);
}

#[test]
fn a_command_line_that_clap_refuses_prints_indented_with_a_pretty_layout() {
    let ran = finish(&PRETTY_GUIDE, &["lst", "--json"], PLANETS, Ok(Vec::new()));

    assert_eq!(ran.code, ExitCode::from(2));
    assert!(ran.out.starts_with("{\n  \"error\": {\n"), "{}", ran.out);
    let document: Value = serde_json::from_str(&ran.out).unwrap();
    assert_eq!(document["error"]["code"], "crumbkit.cli.usage");
}

#[test]
fn a_json_stream_keeps_one_line_for_each_row_and_failure_with_a_pretty_layout() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let (_, mut output) = parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        &PRETTY_GUIDE,
        Spelling::DOTS,
        ["tool", "list", "--json"],
    )
    .unwrap();
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::DOTS, PLANETS_STREAM).unwrap();
    let _ = streaming.item(&Planet { name: "Mars" });
    let _ = streaming.failure(&Missing);
    let code = streaming.end(Ok::<(), Missing>(()));

    assert_eq!(code, ExitCode::FAILURE);
    let out = String::from_utf8(out).unwrap();
    let expected = concat!(
        r#"{"schema":"solar/planets-stream/v1","data":{"name":"Mars"}}"#,
        "\n"
    );
    assert_eq!(out, format!("{expected}{ERROR_DOCUMENT}"));
}

#[test]
fn a_tool_that_parses_with_clap_alone_gets_the_form_through_output_new() {
    let view = Tool::try_parse_from(["tool", "list", "--json"])
        .unwrap()
        .view;
    let mut output = crate::Output::new(
        view,
        Tool::command(),
        &ERROR_GUIDE,
        ["tool", "list", "--json"],
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<Planet>, Missing> = Err(Missing);

    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::DOTS,
        PLANETS,
        result,
    );

    assert_eq!(code, 1);
    assert_eq!(String::from_utf8(out).unwrap(), ERROR_DOCUMENT);
}

#[test]
fn a_form_that_does_not_implement_read_reads_no_failure_from_its_own_run() {
    let ran = fail(&ERROR_GUIDE, &["--json"]);

    assert_eq!(ErrorForm.read(&ran.out, &ran.err), None);
}

#[test]
fn a_form_that_implements_read_reads_back_the_code_and_steps_of_the_failure_it_printed() {
    let ran = fail(&LINE_GUIDE, &[]);

    let read = LineForm.read(&ran.out, &ran.err).unwrap();

    assert_eq!(read.code(), "solar.planets.not_found");
    assert_eq!(read.steps(), [Step::Retry]);
    assert_eq!(
        read,
        ReadFailure::new("solar.planets.not_found".to_owned(), vec![Step::Retry])
    );
}

#[test]
fn a_read_failure_keeps_the_fields_it_is_given_with_their_values_and_has_none_without_them() {
    let fields = vec![
        ("id".to_owned(), FieldValue::Unsigned(7)),
        ("found".to_owned(), FieldValue::Null),
        ("tags".to_owned(), FieldValue::list(["red".to_owned()])),
    ];

    let read = ReadFailure::new("solar.planets.not_found".to_owned(), Vec::new())
        .with_fields(fields.clone());

    assert_eq!(read.fields(), fields);
    assert_eq!(
        ReadFailure::new("solar.planets.not_found".to_owned(), Vec::new()).fields(),
        []
    );
}

/// The code that `RenamingForm` writes in place of crumbkit's usage code.
const TOOL_USAGE: &str = "solar.cli.usage";

/// crumbkit's usage code in the dotted spelling of the tests.
const CRUMBKIT_USAGE: &str = "crumbkit.cli.usage";

/// A form that writes the tool's code in place of crumbkit's usage code, in
/// the JSON and in the text, and maps it back when it reads a run.
struct RenamingForm;

impl RenamingForm {
    /// Returns the code that the form writes for `failure`.
    fn code<'a>(failure: &'a PreparedFailure<'_>) -> &'a str {
        if failure.code() == CRUMBKIT_USAGE {
            TOOL_USAGE
        } else {
            failure.code()
        }
    }
}

impl FailureForm for RenamingForm {
    fn document(&self) -> Document<()> {
        ErrorForm.document()
    }

    fn schema(&self) -> Schema {
        ErrorForm.schema()
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> Value {
        json!({ "error": { "code": Self::code(failure), "message": failure.message() } })
    }

    fn write_text(&self, err: &mut dyn Write, failure: &PreparedFailure<'_>) -> io::Result<()> {
        writeln!(err, "{}: {}", Self::code(failure), failure.message())
    }

    fn read(&self, out: &str, err: &str) -> Option<ReadFailure> {
        let code = match out.lines().last() {
            Some(line) => serde_json::from_str::<Value>(line).ok()?["error"]["code"]
                .as_str()?
                .to_owned(),
            None => err.lines().last()?.split_once(": ")?.0.to_owned(),
        };
        let code = if code == TOOL_USAGE {
            CRUMBKIT_USAGE.to_owned()
        } else {
            code
        };
        Some(ReadFailure::new(code, Vec::new()))
    }
}

const RENAMING_GUIDE: Guide = Guide::new().failure_form(&RenamingForm);

#[test]
fn a_form_can_write_its_own_code_in_place_of_crumbkits_usage_code_and_read_maps_it_back() {
    let json = finish(&RENAMING_GUIDE, &["lst", "--json"], PLANETS, Ok(Vec::new()));
    let text = finish(&RENAMING_GUIDE, &["lst"], PLANETS, Ok(Vec::new()));

    assert_eq!(json.code, ExitCode::from(2));
    let document: Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(document["error"]["code"], TOOL_USAGE);
    assert!(text.err.starts_with("solar.cli.usage: "), "{}", text.err);
    for ran in [json, text] {
        let read = RenamingForm.read(&ran.out, &ran.err).unwrap();
        assert_eq!(read.code(), CRUMBKIT_USAGE);
    }
}
