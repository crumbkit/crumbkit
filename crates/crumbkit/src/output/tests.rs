//! Tests for the output flags and `finish_to`.

use std::io::{self, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::ser::Error as _;
use serde::{Serialize, Serializer};

use super::{Output, finish_to};
use crate::{Call, Document, Error, Listing, Row, Rows, Sink, Spelling};

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

const PLANETS: Document<Vec<Planet>> = Document::new("solar", "planets", 1);
const PLANET: Document<Planet> = Document::new("solar", "planet", 1);
const PLANET_LISTING: Document<Listing<Planet>> = Document::new("solar", "planet-listing", 1);

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

/// Parses `tool list` followed by `flags`.
fn parse(flags: &[&str]) -> Result<Output, clap::Error> {
    let args = ["tool", "list"].iter().chain(flags);
    Tool::try_parse_from(args).map(|tool| tool.output)
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Parses `flags`, sets the tool's command as its `main` would, and finishes
/// `value` into memory.
fn finish_with<T: Rows>(flags: &[&str], document: Document<T>, value: T) -> Finished {
    let mut output = parse(flags).unwrap();
    output.set_command(Tool::command());
    finish_output(&output, document, value)
}

/// Finishes `value` into memory as an `Ok` result with the flags and command
/// in `output`.
fn finish_output<T: Rows>(output: &Output, document: Document<T>, value: T) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let result: Result<T, Error> = Ok(value);
    let code = finish_to(&mut sink, output, Spelling::COLONS, document, result);
    Finished {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns a listing of Mercury and Saturn out of `total` planets.
fn two_of(total: usize) -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], total)
}

/// Returns a listing of Mercury and Saturn out of `total` planets whose next
/// step is `tool show <NAME>...`.
fn two_of_with_next(total: usize) -> Listing<Planet> {
    two_of(total).next(Call::new(&["show"]).arg("names"), "shows planets by name.")
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

    assert_eq!(repeated.fields, ["au", "name", "moons"]);
}

#[test]
fn an_unknown_field_is_a_refusal_that_names_the_valid_fields_and_exits_with_code_2() {
    let finished = finish_with(&["--fields", "name,radius"], PLANETS, vec![saturn()]);

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    let expected = "\
error: --fields names an unknown field 'radius'; the valid fields are au, moons, name, ringed
  help:  run `tool -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(finished.err, expected);
}

#[test]
fn an_unknown_field_is_refused_in_json_even_when_the_result_has_no_rows() {
    let finished = finish_with(&["--json", "--fields", "radius"], PLANETS, Vec::new());

    assert_eq!(finished.code, ExitCode::from(2));
    let failure = r#"{"schema":"crumbkit/failure/v1""#;
    assert!(finished.out.starts_with(failure));
}

#[test]
fn the_flags_parse_before_and_after_the_subcommand() {
    let before = Tool::try_parse_from(["tool", "--json", "list", "--long"]).unwrap();

    assert!(before.output.json);
    assert!(before.output.long);
}

#[test]
fn fields_conflicts_with_long_and_with_full() {
    let with_long = parse(&["--fields", "name", "--long"]).unwrap_err();
    let with_full = parse(&["--json", "--fields", "name", "--full"]).unwrap_err();

    assert_eq!(with_long.kind(), ErrorKind::ArgumentConflict);
    assert_eq!(with_full.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn full_requires_json() {
    let error = parse(&["--full"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
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
        &Output::default(),
        Spelling::COLONS,
        PLANETS,
        result,
    );

    assert_eq!(code, ExitCode::FAILURE);
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
        &Output::default(),
        Spelling::COLONS,
        PLANETS,
        result,
    );

    assert_eq!(code, ExitCode::SUCCESS);
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
    const BROKEN: Document<Unserializable> = Document::new("broken", "row", 1);

    let finished = finish_with(&[], BROKEN, Unserializable { id: 7 });

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert_eq!(
        finished.err,
        "error: cannot write to standard output: row 7 cannot be serialized\n"
    );
}

#[test]
fn a_listing_that_shows_fewer_rows_than_the_total_ends_with_a_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(8));

    let expected = "\
NAME     MOONS
Mercury  0
Saturn   274
Showing 2 of 8. --all shows every one.
";
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn the_footer_names_the_rows_with_the_listings_description() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(8).described("planets"));

    assert!(
        finished
            .out
            .ends_with("Saturn   274\nShowing 2 of 8 planets. --all shows every one.\n"),
        "{}",
        finished.out
    );
}

#[test]
fn a_listing_that_shows_every_row_has_no_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of(2).described("planets"));

    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
}

#[test]
fn the_footer_follows_the_table_with_long() {
    let finished = finish_with(&["--long"], PLANET_LISTING, two_of(8));

    let expected = "\
NAME     MOONS  RINGED
Mercury  0      false
Saturn   274    true
Showing 2 of 8. --all shows every one.
";
    assert_eq!(finished.out, expected);
}

#[test]
fn the_footer_follows_the_table_with_fields() {
    let finished = finish_with(&["--fields", "au"], PLANET_LISTING, two_of(8));

    assert_eq!(
        finished.out,
        "AU\n0.39\n9.54\nShowing 2 of 8. --all shows every one.\n"
    );
}

#[test]
fn a_listings_json_has_the_total_beside_the_data_and_no_description() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of(8).described("planets"));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"total":8}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_has_the_total_when_every_row_is_shown() {
    let finished = finish_with(&["--json"], PLANET_LISTING, Listing::new(Vec::new(), 0));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[],"total":0}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_has_the_total_with_full() {
    let listing = Listing::new(vec![saturn()], 8);
    let finished = finish_with(&["--json", "--full"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Saturn","moons":274,"ringed":true,"au":9.54}],"total":8}"#,
            "\n"
        )
    );
}

#[test]
fn a_listings_json_has_the_total_with_fields() {
    let listing = Listing::new(vec![saturn()], 8);
    let finished = finish_with(&["--json", "--fields", "au"], PLANET_LISTING, listing);

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"au":9.54}],"total":8}"#,
            "\n"
        )
    );
}

#[test]
fn a_listing_with_a_next_step_ends_with_the_next_line_after_the_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_with_next(8));

    let expected = "\
NAME     MOONS
Mercury  0
Saturn   274
Showing 2 of 8. --all shows every one.
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, expected);
    assert_eq!(finished.err, "");
}

#[test]
fn the_next_line_follows_the_table_when_there_is_no_footer() {
    let finished = finish_with(&[], PLANET_LISTING, two_of_with_next(2));

    let expected = "\
NAME     MOONS
Mercury  0
Saturn   274
Next: tool show <NAME>... shows planets by name.
";
    assert_eq!(finished.out, expected);
}

#[test]
fn the_next_line_follows_the_footer_with_long() {
    let finished = finish_with(&["--long"], PLANET_LISTING, two_of_with_next(8));

    assert!(
        finished.out.ends_with(
            "Saturn   274    true\nShowing 2 of 8. --all shows every one.\nNext: tool show <NAME>... shows planets by name.\n"
        ),
        "{}",
        finished.out
    );
}

#[test]
fn the_next_line_follows_the_footer_with_fields() {
    let finished = finish_with(&["--fields", "au"], PLANET_LISTING, two_of_with_next(8));

    assert_eq!(
        finished.out,
        "AU\n0.39\n9.54\nShowing 2 of 8. --all shows every one.\nNext: tool show <NAME>... shows planets by name.\n"
    );
}

#[test]
fn json_output_has_no_next_line_and_no_next_field() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of_with_next(8));

    assert_eq!(
        finished.out,
        concat!(
            r#"{"schema":"solar/planet-listing/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"total":8}"#,
            "\n"
        )
    );
}

#[test]
fn a_listing_prints_no_next_line_when_no_command_is_set() {
    let output = parse(&[]).unwrap();

    let finished = finish_output(&output, PLANET_LISTING, two_of_with_next(8));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(
        finished
            .out
            .ends_with("Showing 2 of 8. --all shows every one.\n"),
        "{}",
        finished.out
    );
    assert_eq!(finished.err, "");
}

#[test]
fn a_next_step_whose_call_does_not_resolve_is_left_out_and_the_exit_code_is_0() {
    let listing = two_of(8).next(Call::new(&["remove"]), "removes a planet.");

    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert!(
        finished
            .out
            .ends_with("Showing 2 of 8. --all shows every one.\n"),
        "{}",
        finished.out
    );
    assert_eq!(finished.err, "");
}
