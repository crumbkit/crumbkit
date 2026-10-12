//! Tests for how `finish_to` prints the result of a document without
//! crumbkit's envelope.

use std::fmt;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use clap::CommandFactory as _;

use super::{
    PLANET, PLANET_LISTING, PLANETS, Planet, Tool, finish_with, mercury, parse_against, saturn,
};
use crate::{
    Batch, Call, Code, Document, Error, Failure, Listing, Prose, Recovery, Sink, Spelling,
    finish_to,
};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A failure that names a planet the solar system lacks.
#[derive(Debug)]
struct Missing(&'static str);

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the solar system has no planet {}", self.0)
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

/// The answer of a command that counts planets, in the shape of an existing
/// document.
#[derive(Serialize, JsonSchema)]
struct Count {
    /// The number of planets.
    planets: u32,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The solar system has {} planets.", self.planets)
    }
}

#[test]
fn json_prints_a_list_without_the_envelope_and_with_every_field() {
    let finished = finish_with(
        &["--json"],
        PLANETS.without_envelope(),
        vec![mercury(), saturn()],
    );

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        concat!(
            r#"[{"name":"Mercury","moons":0,"ringed":false,"au":0.39},"#,
            r#"{"name":"Saturn","moons":274,"ringed":true,"au":9.54}]"#,
            "\n"
        )
    );
}

#[test]
fn json_prints_a_single_row_without_the_envelope_as_an_object() {
    let finished = finish_with(&["--json"], PLANET.without_envelope(), saturn());

    assert_eq!(
        finished.out,
        concat!(
            r#"{"name":"Saturn","moons":274,"ringed":true,"au":9.54}"#,
            "\n"
        )
    );
}

#[test]
fn fields_select_the_fields_of_an_unwrapped_document() {
    let list = finish_with(
        &["--json", "--fields", "au,name"],
        PLANETS.without_envelope(),
        vec![saturn()],
    );
    let single = finish_with(
        &["--json", "--fields", "moons"],
        PLANET.without_envelope(),
        saturn(),
    );

    assert_eq!(list.out, "[{\"au\":9.54,\"name\":\"Saturn\"}]\n");
    assert_eq!(single.out, "{\"moons\":274}\n");
}

#[test]
fn json_prints_one_page_of_an_unwrapped_listing_without_its_total() {
    let listing = Listing::from_all(vec![mercury(), saturn()], Some(1)).described("planets");

    let finished = finish_with(&["--json"], PLANET_LISTING.without_envelope(), listing);

    assert_eq!(
        finished.out,
        "[{\"name\":\"Mercury\",\"moons\":0,\"ringed\":false,\"au\":0.39}]\n"
    );
}

#[test]
fn text_output_of_an_unwrapped_listing_prints_its_footer_and_next_step_on_standard_error() {
    let listing = Listing::from_all(vec![mercury(), saturn()], Some(1))
        .described("planets")
        .next(Call::new(&["show"]).arg("names"), "shows each planet.");

    let finished = finish_with(&[], PLANET_LISTING.without_envelope(), listing);

    let expected = "\
Showing 1 of 2 planets. --all shows every one.
Next: tool show <NAME>... shows each planet.
";
    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\n");
    assert_eq!(finished.err, expected);
}

#[test]
fn the_failures_of_an_unwrapped_batch_print_as_text_on_standard_error_with_json() {
    const SHOW: Document<Batch<Planet, Error>> =
        Document::new("solar", "planets-show", 1, "The test rows.").without_envelope();
    let batch = Batch::new(vec![saturn()], vec![Missing("Vulcan").into()]);

    let finished = finish_with(&["--json"], SHOW, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.out,
        "[{\"name\":\"Saturn\",\"moons\":274,\"ringed\":true,\"au\":9.54}]\n"
    );
    let expected = "\
error: the solar system has no planet Vulcan
  help:  run `tool list`
  code:  solar::planets::not_found
";
    assert_eq!(finished.err, expected);
}

#[test]
fn json_prints_the_value_of_an_unwrapped_prose_result_and_its_failures_on_standard_error() {
    const COUNT: Document<Prose<Count>> =
        Document::new("solar", "planets-count", 2, "The number of planets.").without_envelope();
    let prose = Prose::with_failures(Count { planets: 8 }, vec![Missing("Vulcan").into()]);

    let finished = finish_with(&["--json"], COUNT, prose);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "{\"planets\":8}\n");
    assert!(
        finished
            .err
            .starts_with("error: the solar system has no planet Vulcan\n")
    );
}

#[test]
fn a_failed_unwrapped_command_prints_the_failure_document_with_json() {
    let mut output = parse_against(Tool::command(), &["--json"]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<Planet>, Error> = Err(Missing("Vulcan").into());

    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        PLANETS.without_envelope(),
        result,
    );

    assert_eq!(code, ExitCode::FAILURE);
    let out = String::from_utf8(out).unwrap();
    assert!(
        out.starts_with(r#"{"schema":"crumbkit/failure/v1","code":"solar::planets::not_found""#)
    );
}
