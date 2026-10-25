//! Tests for a row type that sets `Row::SPACED`, whose text prints a blank
//! line after a row that printed child lines when another row follows
//! (ADR-0151).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Tool, built_with, finish_output, finish_with};
use crate::help::Guide;
use crate::{Batch, Document, Error, Listing, Row};

/// A service and the processes it runs, each of which prints under the
/// service, with a blank line between services.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: String,
    /// Whether the service answers.
    state: &'static str,
    /// Where the service runs.
    region: &'static str,
    /// The processes that the service runs.
    processes: Vec<String>,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name", "state"];
    const LONG_COLUMNS: &'static [&'static str] = &["region"];
    const CHILDREN: Option<&'static str> = Some("processes");
    const SPACED: bool = true;
}

const SERVICES: Document<Vec<Service>> =
    Document::new("ops", "spaced-services", 1, "The test services.");
const SERVICE_LISTING: Document<Listing<Service>> =
    Document::new("ops", "spaced-service-listing", 1, "The test services.");
const SERVICE_BATCH: Document<Batch<Service, Error>> =
    Document::new("ops", "spaced-service-batch", 1, "The test services.");

fn service(name: &str, processes: &[&str]) -> Service {
    Service {
        name: name.to_owned(),
        state: "up",
        region: "eu",
        processes: processes.iter().map(|&name| name.to_owned()).collect(),
    }
}

fn api_and_web() -> Vec<Service> {
    vec![
        service("api", &["server", "worker"]),
        service("web", &["assets"]),
    ]
}

#[test]
fn a_spaced_row_type_prints_a_blank_line_between_a_row_with_child_lines_and_the_next_row() {
    let finished = finish_with(&[], SERVICES, api_and_web());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\n  server\n  worker\n\nweb   up\n  assets\n"
    );
}

#[test]
fn a_spaced_row_type_prints_no_blank_line_after_a_row_without_child_lines() {
    let rows = vec![
        service("api", &[]),
        service("web", &["assets"]),
        service("db", &[]),
    ];
    let finished = finish_with(&[], SERVICES, rows);

    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\nweb   up\n  assets\n\ndb    up\n"
    );
}

#[test]
fn a_spaced_row_type_prints_no_blank_line_after_its_last_row() {
    let finished = finish_with(&[], SERVICES, vec![service("api", &["server"])]);

    assert_eq!(finished.out, "NAME  STATE\napi   up\n  server\n");
}

#[test]
fn a_spaced_row_type_prints_the_blank_line_with_long() {
    let finished = finish_with(&["--long"], SERVICES, api_and_web());

    assert_eq!(
        finished.out,
        "NAME  STATE  REGION\napi   up     eu\n  server\n  worker\n\nweb   up     eu\n  assets\n"
    );
}

#[test]
fn a_spaced_row_type_prints_no_blank_line_with_fields() {
    let finished = finish_with(&["--fields", "name"], SERVICES, api_and_web());

    assert_eq!(finished.out, "NAME\napi\nweb\n");
}

#[test]
fn a_spaced_row_type_prints_the_json_that_an_unspaced_row_type_prints() {
    let finished = finish_with(&["--json"], SERVICES, api_and_web());

    assert_eq!(
        finished.out,
        "{\"schema\":\"ops/spaced-services/v1\",\"data\":[\
         {\"name\":\"api\",\"state\":\"up\",\"processes\":[\"server\",\"worker\"]},\
         {\"name\":\"web\",\"state\":\"up\",\"processes\":[\"assets\"]}]}\n"
    );
}

/// A service whose text is its own line, with its processes under that
/// line and a blank line between services.
#[derive(Serialize, JsonSchema)]
struct Lined {
    /// The service's name.
    name: String,
    /// The processes that the service runs.
    processes: Vec<String>,
}

impl Row for Lined {
    const COLUMNS: &'static [&'static str] = &["name"];
    const LINE: Option<&'static str> = Some("service <NAME>");
    const CHILDREN: Option<&'static str> = Some("processes");
    const SPACED: bool = true;

    fn text_line(&self) -> Option<String> {
        Some(format!("service {}", self.name))
    }
}

const LINED: Document<Vec<Lined>> = Document::new("ops", "spaced-lined", 1, "The test services.");
const LINED_BATCH: Document<Batch<Lined, Error>> =
    Document::new("ops", "spaced-lined-batch", 1, "The test services.");

fn lined(name: &str, processes: &[&str]) -> Lined {
    Lined {
        name: name.to_owned(),
        processes: processes.iter().map(|&name| name.to_owned()).collect(),
    }
}

#[test]
fn a_spaced_row_type_with_its_own_text_line_prints_a_blank_line_after_a_row_with_child_lines() {
    let rows = vec![
        lined("api", &["server"]),
        lined("web", &[]),
        lined("db", &["primary"]),
    ];
    let finished = finish_with(&[], LINED, rows);

    assert_eq!(
        finished.out,
        "service api\n  server\n\nservice web\nservice db\n  primary\n"
    );
}

#[test]
fn a_spaced_listing_that_the_budget_cuts_prints_no_blank_line_after_its_last_printed_row() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    // Each row's child line takes about 600 bytes, so only the first row
    // fits in 1,024 bytes with its child line.
    let long = "x".repeat(600);
    let rows = vec![service("api", &[&long]), service("web", &[&long])];
    let finished = finish_output(output, SERVICE_LISTING, Listing::new(rows, 2));

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, format!("NAME  STATE\napi   up\n  {long}\n"));
    assert!(finished.err.contains("1 of 2"), "{}", finished.err);
}

/// Returns a batch of `tool show` that holds `rows`, one for each key.
fn answered<R: Row>(rows: Vec<R>, keys: &[&str]) -> Batch<R, Error> {
    let mut batch = Batch::new(&["show"], "names", keys);
    batch.extend(rows.into_iter().map(Ok));
    batch
}

#[test]
fn room_counts_the_blank_line_between_spaced_rows_in_a_table() {
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    let rows = vec![
        service("api", &["server"]),
        service("web", &[]),
        service("db", &["primary"]),
    ];
    let batch = answered(rows, &["api", "web", "db"]);
    let mut room = output.room(SERVICE_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, SERVICE_BATCH, batch);

    assert!(finished.out.contains("\n\nweb"), "{}", finished.out);
    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
}

#[test]
fn room_counts_the_blank_line_between_spaced_rows_with_their_own_text_line() {
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    let rows = vec![lined("api", &["server"]), lined("web", &[])];
    let batch = answered(rows, &["api", "web"]);
    let mut room = output.room(LINED_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, LINED_BATCH, batch);

    assert_eq!(finished.out, "service api\n  server\n\nservice web\n");
    assert_eq!(room.printed(), finished.out.len());
}

const LINED_LISTING: Document<Listing<Lined>> =
    Document::new("ops", "spaced-lined-listing", 1, "The test services.");

#[test]
fn a_spaced_listing_with_its_own_text_line_that_the_budget_cuts_prints_no_blank_line_after_its_last_printed_row()
 {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    // Each row's child line takes about 600 bytes, so only the first row
    // fits in 1,024 bytes with its child line.
    let long = "x".repeat(600);
    let rows = vec![lined("api", &[&long]), lined("web", &[&long])];
    let finished = finish_output(output, LINED_LISTING, Listing::new(rows, 2));

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, format!("service api\n  {long}\n"));
    assert!(finished.err.contains("1 of 2"), "{}", finished.err);
}

/// A service whose processes print under it only with `--long`, with a
/// blank line between services.
#[derive(Serialize, JsonSchema)]
struct Detailed {
    /// The service's name.
    name: String,
    /// Where the service runs.
    region: &'static str,
    /// The processes that the service runs.
    processes: Vec<String>,
}

impl Row for Detailed {
    const COLUMNS: &'static [&'static str] = &["name"];
    const LONG_COLUMNS: &'static [&'static str] = &["region"];
    const CHILDREN: Option<&'static str> = Some("processes");
    const SPACED: bool = true;

    fn child_lines(&self) -> Vec<String> {
        Vec::new()
    }

    fn long_child_lines(&self) -> Vec<String> {
        self.processes.clone()
    }
}

const DETAILED: Document<Vec<Detailed>> =
    Document::new("ops", "spaced-detailed", 1, "The test services.");
const DETAILED_BATCH: Document<Batch<Detailed, Error>> =
    Document::new("ops", "spaced-detailed-batch", 1, "The test services.");

fn detailed() -> Vec<Detailed> {
    ["api", "web"]
        .into_iter()
        .map(|name| Detailed {
            name: name.to_owned(),
            region: "eu",
            processes: vec![format!("{name}-server")],
        })
        .collect()
}

#[test]
fn a_spaced_row_type_whose_child_lines_print_only_with_long_spaces_its_rows_only_with_long() {
    let compact = finish_with(&[], DETAILED, detailed());
    let long = finish_with(&["--long"], DETAILED, detailed());

    assert_eq!(compact.out, "NAME\napi\nweb\n");
    assert_eq!(
        long.out,
        "NAME  REGION\napi   eu\n  api-server\n\nweb   eu\n  web-server\n"
    );
}

#[test]
fn room_counts_the_blank_line_between_spaced_rows_with_long() {
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list", "--long"],
    );
    let batch = answered(detailed(), &["api", "web"]);
    let mut room = output.room(DETAILED_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, DETAILED_BATCH, batch);

    assert!(finished.out.contains("\n\nweb"), "{}", finished.out);
    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
}
