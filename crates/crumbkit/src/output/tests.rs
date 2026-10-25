//! Tests for the output flags and `finish_to`.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::ser::Error as _;
use serde::{Serialize, Serializer};

use super::{Output, finish_to};
use crate::help::Guide;
use crate::twins::assert_one_word_apart;
use crate::view::{Selection, View};
use crate::{Document, Error, Listing, Row, Rows, Sink, Stop, ViewFlags};

mod about;
mod adds_nothing;
mod batch;
mod budget;
mod built;
mod bytes;
mod capped;
mod children;
mod direct;
mod empty;
mod empty_columns;
mod exits;
mod failed;
mod fitted;
mod headed;
mod layout;
mod lines;
mod listing;
mod long_note;
mod notes;
mod only_problems;
mod plugin;
mod problems;
mod prose;
#[cfg(feature = "miette")]
mod report;
mod roles;
mod spaced;
mod spelled;
mod styled;
mod summary;
mod unwrapped;
mod variants;
mod views;
mod whole;

/// A row with two compact columns, one long column and one field that only
/// `--full` shows. The last field is renamed, so its column name is `au`.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
    moons: u32,
    ringed: bool,
    #[serde(rename = "au")]
    distance: f64,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name", "moons"];
    const LONG_COLUMNS: &'static [&'static str] = &["ringed"];
}

const PLANETS: Document<Vec<Planet>> = Document::new("solar", "planets", 1, "The test rows.");
const PLANET: Document<Planet> = Document::new("solar", "planet", 1, "The test rows.");
const PLANET_LISTING: Document<Listing<Planet>> =
    Document::new("solar", "planet-listing", 1, "The test rows.");

fn mercury() -> Planet {
    Planet {
        name: "Mercury",
        moons: 0,
        ringed: false,
        distance: 0.39,
    }
}

fn saturn() -> Planet {
    Planet {
        name: "Saturn",
        moons: 274,
        ringed: true,
        distance: 9.54,
    }
}

/// A tool with two subcommands, so the tests can put the flags after one and
/// a listing can name the other as its next step.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
    Show {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },
}

/// Parses `tool list` followed by `flags` with clap alone, and returns the
/// output that a tool's `main` builds from the parsed view flags.
fn parse(flags: &[&str]) -> Result<Output, clap::Error> {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    let tool = Tool::try_parse_from(&args)?;
    Ok(Output::new(
        tool.view,
        crate::DryRun::default(),
        Tool::command(),
        &Guide::new(),
        args,
    )
    .unwrap())
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// What `Output::new_to` returned and wrote: the output of a run whose view
/// flags work together, or the stop of a refused pair.
struct Built {
    result: Result<Output, Stop>,
    out: String,
    err: String,
}

/// Parses `tool list` followed by `flags` with clap alone, and builds the
/// output into memory as a tool's `main` builds it with `Output::new_to`.
fn build_to(flags: &[&str]) -> Built {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    let tool = Tool::try_parse_from(&args).unwrap();
    build_output(tool.view, &args)
}

/// Builds the output of the test tool from `view` and `args` into memory,
/// as a tool's `main` builds it with `Output::new_to`.
fn build_output(view: ViewFlags, args: &[&str]) -> Built {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let dry_run = crate::DryRun::default();
    let result = Output::new_to(
        &mut sink,
        view,
        dry_run,
        Tool::command(),
        &Guide::new(),
        args,
    );
    Built {
        result,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Parses `tool list` followed by `flags` against `command` with clap
/// alone, and returns the output that holds that command, as a tool's
/// `main` builds it.
fn parse_against(command: clap::Command, flags: &[&str]) -> Output {
    let args: Vec<&str> = ["tool", "list"].iter().chain(flags).copied().collect();
    built_with(command, &Guide::new(), &args)
}

/// Parses `args` against `command` with clap alone, and returns the output
/// that a tool's `main` builds from the parsed view flags, `command` and
/// `guide`.
fn built_with(mut command: clap::Command, guide: &Guide, args: &[&str]) -> Output {
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let view = ViewFlags::from_arg_matches(&matches).unwrap();
    Output::new(view, crate::DryRun::default(), command, guide, args).unwrap()
}

/// Parses `flags`, builds the output as a tool that parses with clap alone
/// builds it, and finishes `value` into memory.
fn finish_with<T: Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    let output = parse_against(Tool::command(), flags);
    finish_output(output, document, value)
}

/// Finishes `value` into memory as an `Ok` result with the flags and command
/// in `output`.
fn finish_output<T: Rows>(output: Output, document: Document<T>, value: T) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let result: Result<T, Error> = Ok(value);
    let code = finish_to(&mut sink, output, document, result);
    Finished {
        code: ExitCode::from(code),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn text_output_is_a_table_of_the_compact_columns() {
    let finished = finish_with(&[], PLANETS, vec![mercury(), saturn()]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    let expected = "\
NAME     MOONS
Mercury  0
Saturn   274
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn a_single_row_prints_as_a_table_with_one_row() {
    let finished = finish_with(&[], PLANET, saturn());

    assert_eq!(finished.out, "NAME    MOONS\nSaturn  274\n");
}

#[test]
fn long_adds_the_long_columns_after_the_compact_ones_in_text() {
    let finished = finish_with(&["--long"], PLANETS, vec![mercury(), saturn()]);

    let expected = "\
NAME     MOONS  RINGED
Mercury  0      false
Saturn   274    true
";
    assert_eq!(finished.out, expected);
}

#[test]
fn json_prints_the_schema_id_and_the_compact_columns_on_one_line() {
    let finished = finish_with(&["--json"], PLANETS, vec![mercury(), saturn()]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}]}"#,
            "\n"
        )
    );
}

#[test]
fn long_adds_the_long_columns_in_json() {
    let finished = finish_with(&["--json", "--long"], PLANETS, vec![saturn()]);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets/v1","data":[{"name":"Saturn","moons":274,"ringed":true}]}"#,
            "\n"
        )
    );
}

#[test]
fn full_prints_every_field_in_declaration_order() {
    let finished = finish_with(&["--json", "--full"], PLANETS, vec![saturn()]);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets/v1","data":[{"name":"Saturn","moons":274,"ringed":true,"au":9.54}]}"#,
            "\n"
        )
    );
}

#[test]
fn a_single_rows_data_is_an_object_in_compact_and_full_json() {
    let compact = finish_with(&["--json"], PLANET, saturn());
    let full = finish_with(&["--json", "--full"], PLANET, saturn());

    assert_eq!(
        compact.out,
        concat!(
            r#"{"schema":"solar/planet/v1","data":{"name":"Saturn","moons":274}}"#,
            "\n"
        )
    );
    assert_eq!(
        full.out,
        concat!(
            r#"{"schema":"solar/planet/v1","data":{"name":"Saturn","moons":274,"ringed":true,"au":9.54}}"#,
            "\n"
        )
    );
}

#[test]
fn a_lists_data_is_an_array_even_when_it_holds_one_row_or_none() {
    let one = finish_with(&["--json"], PLANETS, vec![saturn()]);
    let none = finish_with(&["--json"], PLANETS, Vec::new());

    assert!(one.out.contains(r#""data":[{"#), "{}", one.out);
    assert_eq!(
        none.out,
        concat!(r#"{"schema":"solar/planets/v1","data":[]}"#, "\n")
    );
}

#[test]
fn fields_prints_exactly_the_named_fields_in_the_given_order_in_text() {
    let finished = finish_with(&["--fields", "au,name"], PLANETS, vec![mercury(), saturn()]);

    let expected = "\
AU    NAME
0.39  Mercury
9.54  Saturn
";
    assert_eq!(finished.out, expected);
}

#[test]
fn fields_prints_exactly_the_named_fields_in_the_given_order_in_json() {
    let finished = finish_with(&["--json", "--fields", "au,name"], PLANETS, vec![saturn()]);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planets/v1","data":[{"au":9.54,"name":"Saturn"}]}"#,
            "\n"
        )
    );
}

#[test]
fn fields_accepts_a_repeated_flag_as_well_as_commas() {
    let repeated = parse(&["--fields", "au", "--fields", "name,moons"]).unwrap();

    assert_eq!(repeated.field_list(), ["au", "name", "moons"]);
}

#[test]
fn an_unknown_field_is_a_refusal_whose_hint_names_the_valid_fields_and_exits_with_code_2() {
    let refused = ["--fields", "name,radius"];
    let admitted = ["--fields", "name,au"];
    assert_one_word_apart(&refused, &admitted);
    let twin = finish_with(&admitted, PLANETS, vec![saturn()]);
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&refused, PLANETS, vec![saturn()]);

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    let expected = "\
error: --fields names an unknown field 'radius'
  help:  use the valid fields: au, moons, name, ringed
  code:  tool::cli::usage
";
    assert_eq!(finished.err, expected);
}

#[test]
fn an_unknown_field_is_refused_in_json_even_when_the_result_has_no_rows() {
    let refused = ["--json", "--fields", "radius"];
    let admitted = ["--json", "--fields", "au"];
    assert_one_word_apart(&refused, &admitted);
    let twin = finish_with(&admitted, PLANETS, Vec::new());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&refused, PLANETS, Vec::new());

    assert_eq!(finished.code, ExitCode::from(2));
    let recovery =
        r#""recovery":{"kind":"hint","text":"use the valid fields: au, moons, name, ringed"}"#;
    assert!(finished.out.contains(recovery));
}

#[test]
fn an_unknown_field_refusal_names_the_flag_and_the_unknown_name_under_fields() {
    let refused = ["--json", "--fields", "name,radius"];
    let admitted = ["--json", "--fields", "name,au"];
    assert_one_word_apart(&refused, &admitted);
    let twin = finish_with(&admitted, PLANETS, Vec::new());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&refused, PLANETS, Vec::new());

    assert!(
        finished
            .out
            .contains(r#""fields":{"argument":"--fields","value":"radius"}"#),
        "{}",
        finished.out
    );
}

/// A star, one variant of `Body`, which `Body`'s schema reaches through a
/// `$ref`.
#[derive(Serialize, JsonSchema)]
struct Star {
    name: &'static str,
    kelvin: u32,
}

/// A row type whose schema lists its variants under `anyOf` and has no
/// top-level `properties`.
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
enum Body {
    Star(Star),
    Moon {
        name: &'static str,
        planet: &'static str,
    },
}

impl Row for Body {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const BODIES: Document<Vec<Body>> = Document::new("solar", "bodies", 1, "The test rows.");

#[test]
fn fields_accepts_the_fields_of_each_variant_of_a_row_type_without_top_level_properties() {
    let sun = Body::Star(Star {
        name: "Sun",
        kelvin: 5772,
    });
    let moon = Body::Moon {
        name: "Titan",
        planet: "Saturn",
    };

    let finished = finish_with(&["--fields", "name,kelvin,planet"], BODIES, vec![sun, moon]);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    let expected = "\
NAME   KELVIN  PLANET
Sun    5772    -
Titan  -       Saturn
";
    assert_eq!(finished.out, expected);
}

#[test]
fn the_valid_fields_of_a_row_type_with_variants_are_each_variants_fields_once_in_order() {
    assert_one_word_apart(&["--fields", "radius"], &["--fields", "kelvin"]);
    let twin = finish_with(&["--fields", "kelvin"], BODIES, Vec::new());
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);
    let finished = finish_with(&["--fields", "radius"], BODIES, Vec::new());

    assert_eq!(finished.code, ExitCode::from(2));
    assert!(
        finished
            .err
            .contains("help:  use the valid fields: kelvin, name, planet\n"),
        "{}",
        finished.err
    );
}

#[test]
fn the_flags_parse_before_and_after_the_subcommand() {
    let words = ["tool", "--json", "list", "--long"];
    let view = Tool::try_parse_from(words).unwrap().view;
    let before = Output::new(
        view,
        crate::DryRun::default(),
        Tool::command(),
        &Guide::new(),
        words,
    )
    .unwrap();

    assert_eq!(before.view(), &View::Json(Selection::Long));
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

#[test]
fn a_failed_write_exits_with_code_1_and_reports_the_cause() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(io::ErrorKind::Other), &mut err);
    let result: Result<_, Error> = Ok(vec![saturn()]);

    let code = finish_to(
        &mut sink,
        parse_against(Tool::command(), &[]),
        PLANETS,
        result,
    );

    assert_eq!(code, 1);
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "error: cannot write to standard output: the writer failed\n"
    );
}

#[test]
fn a_reader_that_closes_the_pipe_early_still_exits_with_code_0_and_no_error() {
    let mut err = Vec::new();
    let mut sink = Sink::new(Failing(io::ErrorKind::BrokenPipe), &mut err);
    let result: Result<_, Error> = Ok(vec![saturn()]);

    let code = finish_to(
        &mut sink,
        parse_against(Tool::command(), &[]),
        PLANETS,
        result,
    );

    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(err).unwrap(), "");
}

/// A row whose serialization always fails.
#[derive(JsonSchema)]
struct Unserializable {
    id: u32,
}

impl Serialize for Unserializable {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom(format!(
            "row {} cannot be serialized",
            self.id
        )))
    }
}

impl Row for Unserializable {
    const COLUMNS: &'static [&'static str] = &["id"];
}

#[test]
fn a_result_that_cannot_be_serialized_exits_with_code_1_and_reports_the_cause() {
    const BROKEN: Document<Unserializable> = Document::new("broken", "row", 1, "The test rows.");

    let finished = finish_with(&[], BROKEN, Unserializable { id: 7 });

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert_eq!(
        finished.err,
        "error: cannot write to standard output: row 7 cannot be serialized\n"
    );
}

#[test]
fn is_json_reports_whether_the_caller_typed_json() {
    assert!(parse(&["--json"]).unwrap().is_json());
    assert!(!parse(&[]).unwrap().is_json());
}

#[test]
fn progress_is_never_allowed_with_json() {
    assert!(!parse(&["--json"]).unwrap().progress_allowed());
}
