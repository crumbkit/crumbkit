//! Tests for how `finish_to` prints a batch: its rows, its failures and its document.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    AsFailure, Batch, Call, Code, Document, Error, Failure, Output, Recovery, Row, Sink, Spelling,
    finish_to,
};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A row with two compact columns, one long column and one field that only
/// `--full` shows.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
    moons: u32,
    ringed: bool,
    au: f64,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name", "moons"];
    const LONG_COLUMNS: &'static [&'static str] = &["ringed"];
}

fn mercury() -> Planet {
    Planet {
        name: "Mercury",
        moons: 0,
        ringed: false,
        au: 0.39,
    }
}

fn saturn() -> Planet {
    Planet {
        name: "Saturn",
        moons: 274,
        ringed: true,
        au: 9.54,
    }
}

/// The error in a failure's `source()` chain.
#[derive(Debug)]
struct Unreadable;

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the star chart cannot be read")
    }
}

impl std::error::Error for Unreadable {}

/// A failure that names the planets the solar system lacks. With `charted`
/// set, it has a cause and a url as well.
#[derive(Debug)]
struct Missing {
    names: &'static str,
    charted: bool,
}

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.names)
    }
}

impl std::error::Error for Missing {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.charted
            .then_some(&Unreadable as &(dyn std::error::Error + 'static))
    }
}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["list"]))
    }

    fn url(&self) -> Option<&str> {
        self.charted.then_some("https://example.com/planets")
    }
}

/// Returns a failure with no cause and no url that names `names`.
fn missing(names: &'static str) -> Missing {
    Missing {
        names,
        charted: false,
    }
}

const PLANETS: Document<Batch<Planet, Missing>> =
    Document::new("solar", "planets-show", 1, "The test rows.");

/// A tool with a `list` command that a recovery names and a `show` command
/// that takes many names.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Command {
    List,
    Show {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Parses `tool show Saturn` followed by `flags` and sets the tool's command
/// as its `main` would.
fn output(flags: &[&str]) -> Output {
    let args = ["tool", "show", "Saturn"].iter().chain(flags);
    let mut output = Tool::try_parse_from(args).unwrap().output;
    output.set_command(Tool::command());
    output
}

/// Finishes `batch` into memory with `flags`.
fn finish_with<E: AsFailure>(flags: &[&str], batch: Batch<Planet, E>) -> Finished {
    let document = Document::new("solar", "planets-show", 1, "The test rows.");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(batch);
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &output(flags),
        Spelling::COLONS,
        document,
        result,
    );
    Finished {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns a batch of Mercury and Saturn and the failures `failures`.
fn two_with(failures: Vec<Missing>) -> Batch<Planet, Missing> {
    Batch::new(vec![mercury(), saturn()], failures)
}

#[test]
fn a_batch_without_failures_prints_its_rows_as_a_table_and_exits_with_code_0() {
    let finished = finish_with(&[], two_with(Vec::new()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
    assert_eq!(finished.err, "");
}

#[test]
fn a_batch_prints_its_rows_on_standard_output_and_each_failure_on_standard_error_in_order() {
    let failures = vec![
        missing("Vulcan"),
        Missing {
            names: "Nibiru",
            charted: true,
        },
    ];

    let finished = finish_with(&[], two_with(failures));

    let expected = "\
error: the solar system has no planet Vulcan
  help:  run `tool list`
  code:  solar::planets::not_found
error: the solar system has no planet Nibiru
  cause: the star chart cannot be read
  help:  run `tool list`
  code:  solar::planets::not_found
  url:   https://example.com/planets
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
    assert_eq!(finished.err, expected);
}

#[test]
fn each_failure_in_a_batch_prints_exactly_as_a_failed_command_prints_it() {
    let mut err = Vec::new();
    let failed: Result<Batch<Planet, Missing>, Missing> = Err(missing("Vulcan"));
    finish_to(
        &mut Sink::new(Vec::new(), &mut err),
        &output(&[]),
        Spelling::COLONS,
        PLANETS,
        failed,
    );

    let finished = finish_with(&[], two_with(vec![missing("Vulcan")]));

    assert_eq!(finished.err, String::from_utf8(err).unwrap());
}

#[test]
fn a_batch_without_rows_prints_nothing_on_standard_output_and_only_its_failures() {
    let batch = Batch::new(Vec::new(), vec![missing("Vulcan")]);

    let finished = finish_with(&[], batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert!(
        finished
            .err
            .starts_with("error: the solar system has no planet Vulcan\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_batch_without_rows_or_failures_prints_nothing_and_exits_with_code_0() {
    let batch: Batch<Planet, Missing> = Batch::new(Vec::new(), Vec::new());

    let finished = finish_with(&[], batch);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, "");
}

#[test]
fn a_batch_of_crumbkit_errors_prints_each_as_the_failure_inside() {
    let direct = finish_with(&[], two_with(vec![missing("Vulcan")]));
    let wrapped = finish_with(
        &[],
        Batch::new(
            vec![mercury(), saturn()],
            vec![Error::from(missing("Vulcan"))],
        ),
    );
    let wrapped_json = finish_with(
        &["--json"],
        Batch::new(vec![saturn()], vec![Error::from(missing("Vulcan"))]),
    );

    assert_eq!(wrapped.code, ExitCode::FAILURE);
    assert_eq!(wrapped.out, direct.out);
    assert_eq!(wrapped.err, direct.err);
    assert!(
        wrapped_json
            .out
            .contains(r#""failures":[{"code":"solar::planets::not_found","#),
        "{}",
        wrapped_json.out
    );
}

#[test]
fn long_and_fields_shape_the_rows_and_leave_the_failures_unchanged_in_text() {
    let plain = finish_with(&[], two_with(vec![missing("Vulcan")]));
    let long = finish_with(&["--long"], two_with(vec![missing("Vulcan")]));
    let fields = finish_with(&["--fields", "au"], two_with(vec![missing("Vulcan")]));

    assert_eq!(
        long.out,
        "NAME     MOONS  RINGED\nMercury  0      false\nSaturn   274    true\n"
    );
    assert_eq!(fields.out, "AU\n0.39\n9.54\n");
    assert_eq!(long.err, plain.err);
    assert_eq!(fields.err, plain.err);
    assert_eq!(long.code, ExitCode::FAILURE);
    assert_eq!(fields.code, ExitCode::FAILURE);
}

#[test]
fn json_of_a_batch_without_failures_holds_an_empty_failures_array() {
    let finished = finish_with(&["--json"], two_with(Vec::new()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets-show/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"failures":[]}"#,
            "\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn json_of_a_batch_prints_one_document_with_the_failures_and_exits_with_code_1() {
    let failures = vec![
        missing("Vulcan"),
        Missing {
            names: "Nibiru",
            charted: true,
        },
    ];

    let finished = finish_with(&["--json"], Batch::new(vec![saturn()], failures));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets-show/v1","data":[{"name":"Saturn","moons":274}],"failures":["#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}},"#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Nibiru","cause":"the star chart cannot be read","recovery":{"kind":"run","call":"tool list"},"url":"https://example.com/planets"}"#,
            "]}\n"
        )
    );
    assert_eq!(finished.err, "");
}

#[test]
fn a_failure_in_a_batch_document_is_the_failure_document_without_its_schema() {
    let mut out = Vec::new();
    let failed: Result<Batch<Planet, Missing>, Missing> = Err(Missing {
        names: "Nibiru",
        charted: true,
    });
    finish_to(
        &mut Sink::new(&mut out, Vec::new()),
        &output(&["--json"]),
        Spelling::COLONS,
        PLANETS,
        failed,
    );
    let mut alone: serde_json::Value = serde_json::from_slice(&out).unwrap();
    alone.as_object_mut().unwrap().remove("schema");

    let finished = finish_with(
        &["--json"],
        Batch::new(
            Vec::new(),
            vec![Missing {
                names: "Nibiru",
                charted: true,
            }],
        ),
    );

    let batch: serde_json::Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(batch["failures"][0], alone);
    assert_eq!(batch["data"], serde_json::json!([]));
}

#[test]
fn json_full_shapes_the_data_and_leaves_the_failures_unchanged() {
    let finished = finish_with(
        &["--json", "--full"],
        Batch::new(vec![saturn()], vec![missing("Vulcan")]),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets-show/v1","data":[{"name":"Saturn","moons":274,"ringed":true,"au":9.54}],"failures":["#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}}"#,
            "]}\n"
        )
    );
}

#[test]
fn json_fields_shapes_the_data_and_leaves_the_failures_unchanged() {
    let finished = finish_with(
        &["--json", "--fields", "au"],
        Batch::new(vec![saturn()], vec![missing("Vulcan")]),
    );

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets-show/v1","data":[{"au":9.54}],"failures":["#,
            r#"{"code":"solar::planets::not_found","message":"the solar system has no planet Vulcan","recovery":{"kind":"run","call":"tool list"}}"#,
            "]}\n"
        )
    );
}

#[test]
fn an_unknown_field_exits_with_code_2_before_anything_of_the_batch_prints() {
    let finished = finish_with(&["--fields", "radius"], two_with(vec![missing("Vulcan")]));

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert!(
        finished
            .err
            .starts_with("error: --fields names an unknown field 'radius'")
    );
    assert!(!finished.err.contains("Vulcan"));
}

/// A writer whose every write fails with an error of `kind`.
struct Failing(io::ErrorKind);

impl Write for Failing {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(self.0, "the writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Finishes `batch` into a sink whose output writer fails with `kind`, and
/// returns the exit code and what went to the error writer.
fn finish_failing(
    kind: io::ErrorKind,
    flags: &[&str],
    batch: Batch<Planet, Missing>,
) -> (ExitCode, String) {
    let mut err = Vec::new();
    let result: Result<_, Missing> = Ok(batch);
    let code = finish_to(
        &mut Sink::new(Failing(kind), &mut err),
        &output(flags),
        Spelling::COLONS,
        PLANETS,
        result,
    );
    (code, String::from_utf8(err).unwrap())
}

#[test]
fn a_reader_that_closes_the_pipe_early_on_a_batch_with_failures_leaves_code_1() {
    let (text, text_err) = finish_failing(
        io::ErrorKind::BrokenPipe,
        &[],
        two_with(vec![missing("Vulcan")]),
    );
    let (json, json_err) = finish_failing(
        io::ErrorKind::BrokenPipe,
        &["--json"],
        two_with(vec![missing("Vulcan")]),
    );

    assert_eq!(text, ExitCode::FAILURE);
    assert!(
        text_err.starts_with("error: the solar system has no planet Vulcan\n"),
        "{text_err}"
    );
    assert_eq!(json, ExitCode::FAILURE);
    assert_eq!(json_err, "");
}

#[test]
fn a_reader_that_closes_the_pipe_early_on_a_batch_without_failures_leaves_code_0() {
    let (code, err) = finish_failing(io::ErrorKind::BrokenPipe, &[], two_with(Vec::new()));

    assert_eq!(code, ExitCode::SUCCESS);
    assert_eq!(err, "");
}

#[test]
fn a_failed_write_of_a_batch_exits_with_code_1_and_reports_the_cause_before_the_failures() {
    let (code, err) = finish_failing(io::ErrorKind::Other, &[], two_with(vec![missing("Vulcan")]));

    assert_eq!(code, ExitCode::FAILURE);
    assert!(
        err.starts_with(
            "error: cannot write to standard output: the writer failed\nerror: the solar system has no planet Vulcan\n"
        ),
        "{err}"
    );
}
