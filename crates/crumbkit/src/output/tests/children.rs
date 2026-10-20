//! Tests for rows that print their children as indented lines under them
//! (ADR-0120).

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{Tool, built_with, finish_output, finish_with};
use crate::help::Guide;
use crate::{Batch, Document, Error, Listing, Row};

/// A service and its components, each of which prints under the service.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: String,
    /// Whether the service answers.
    state: &'static str,
    /// Where the service runs.
    region: &'static str,
    /// The parts that the service runs on.
    components: Vec<Component>,
}

/// One part of a service.
#[derive(Serialize, JsonSchema)]
struct Component {
    /// The part's name.
    name: String,
    /// Whether the part answers.
    up: bool,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name", "state"];
    const LONG_COLUMNS: &'static [&'static str] = &["region"];
    const CHILDREN: Option<&'static str> = Some("components");

    fn child_lines(&self) -> Vec<String> {
        self.components
            .iter()
            .map(|component| {
                let mark = if component.up { '+' } else { '-' };
                format!("{mark} {}", component.name)
            })
            .collect()
    }
}

const SERVICES: Document<Vec<Service>> = Document::new("ops", "services", 1, "The test services.");
const SERVICE_LISTING: Document<Listing<Service>> =
    Document::new("ops", "service-listing", 1, "The test services.");
const SERVICE_BATCH: Document<Batch<Service, Error>> =
    Document::new("ops", "service-batch", 1, "The test services.");

fn service(name: &str, components: &[(&str, bool)]) -> Service {
    Service {
        name: name.to_owned(),
        state: "up",
        region: "eu",
        components: components
            .iter()
            .map(|&(name, up)| Component {
                name: name.to_owned(),
                up,
            })
            .collect(),
    }
}

fn api_and_web() -> Vec<Service> {
    vec![
        service("api", &[("database", true), ("cache", false)]),
        service("web", &[("assets", true)]),
    ]
}

#[test]
fn each_row_s_child_lines_print_indented_by_two_spaces_under_its_table_row() {
    let finished = finish_with(&[], SERVICES, api_and_web());

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\n  + database\n  - cache\nweb   up\n  + assets\n"
    );
}

#[test]
fn a_child_line_wider_than_the_table_leaves_the_column_widths_unchanged() {
    let rows = vec![service("api", &[("a-very-long-component-name", true)])];
    let finished = finish_with(&[], SERVICES, rows);

    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\n  + a-very-long-component-name\n"
    );
}

#[test]
fn a_row_without_child_lines_prints_nothing_under_its_table_row() {
    let rows = vec![service("api", &[]), service("web", &[("assets", true)])];
    let finished = finish_with(&[], SERVICES, rows);

    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\nweb   up\n  + assets\n"
    );
}

#[test]
fn a_child_line_prints_its_control_and_format_characters_escaped() {
    let rows = vec![service("api", &[("data\nbase\u{202e}", true)])];
    let finished = finish_with(&[], SERVICES, rows);

    assert_eq!(
        finished.out,
        "NAME  STATE\napi   up\n  + data\\nbase\\u{202e}\n"
    );
}

#[test]
fn long_prints_the_long_columns_and_still_prints_the_child_lines() {
    let finished = finish_with(&["--long"], SERVICES, api_and_web());

    assert_eq!(
        finished.out,
        "NAME  STATE  REGION\napi   up     eu\n  + database\n  - cache\nweb   up     eu\n  + assets\n"
    );
}

#[test]
fn compact_json_holds_the_children_field_after_the_columns() {
    let finished = finish_with(
        &["--json"],
        SERVICES,
        vec![service("web", &[("assets", true)])],
    );

    assert_eq!(
        finished.out,
        "{\"schema\":\"ops/services/v1\",\"data\":[{\"name\":\"web\",\"state\":\"up\",\"components\":[{\"name\":\"assets\",\"up\":true}]}]}\n"
    );
}

#[test]
fn long_json_holds_the_children_field_after_the_long_columns() {
    let rows = vec![service("web", &[("assets", true)])];
    let finished = finish_with(&["--json", "--long"], SERVICES, rows);

    assert!(
        finished.out.contains(
            "{\"name\":\"web\",\"state\":\"up\",\"region\":\"eu\",\"components\":[{\"name\":\"assets\",\"up\":true}]}"
        ),
        "{}",
        finished.out
    );
}

#[test]
fn fields_without_the_children_field_prints_no_child_lines() {
    let finished = finish_with(&["--fields", "name"], SERVICES, api_and_web());

    assert_eq!(finished.out, "NAME\napi\nweb\n");
}

#[test]
fn fields_with_the_children_field_prints_it_as_a_cell_of_compact_json_and_no_child_lines() {
    let rows = vec![service("web", &[("assets", true)])];
    let finished = finish_with(&["--fields", "name,components"], SERVICES, rows);

    assert_eq!(
        finished.out,
        "NAME  COMPONENTS\nweb   [{\"name\":\"assets\",\"up\":true}]\n"
    );
}

#[test]
fn json_fields_without_the_children_field_leaves_it_out() {
    let rows = vec![service("web", &[("assets", true)])];
    let finished = finish_with(&["--json", "--fields", "name"], SERVICES, rows);

    assert_eq!(
        finished.out,
        "{\"schema\":\"ops/services/v1\",\"data\":[{\"name\":\"web\"}]}\n"
    );
}

/// A service whose text prints as its own line, with its components under
/// that line.
#[derive(Serialize, JsonSchema)]
struct Lined {
    /// The service's name.
    name: String,
    /// The parts that the service runs on.
    components: Vec<String>,
}

impl Row for Lined {
    const COLUMNS: &'static [&'static str] = &["name"];
    const LINE: Option<&'static str> = Some("service <NAME>");
    const CHILDREN: Option<&'static str> = Some("components");

    fn text_line(&self) -> Option<String> {
        Some(format!("service {}", self.name))
    }

    fn child_lines(&self) -> Vec<String> {
        self.components.clone()
    }
}

const LINED: Document<Vec<Lined>> = Document::new("ops", "lined", 1, "The test services.");

fn lined(name: &str, components: &[&str]) -> Lined {
    Lined {
        name: name.to_owned(),
        components: components.iter().map(|&name| name.to_owned()).collect(),
    }
}

#[test]
fn child_lines_print_under_a_row_s_own_text_line() {
    let rows = vec![lined("api", &["database", "cache"]), lined("web", &[])];
    let finished = finish_with(&[], LINED, rows);

    assert_eq!(
        finished.out,
        "service api\n  database\n  cache\nservice web\n"
    );
}

#[test]
fn a_listing_that_the_budget_cuts_counts_each_row_s_child_lines() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    // Each table line is short, and each row's child lines take about 600
    // bytes, so only the first row fits in 1,024 bytes with its children.
    let long = "x".repeat(600);
    let rows = vec![
        service("api", &[(&long, true)]),
        service("web", &[(&long, true)]),
    ];
    let finished = finish_output(output, SERVICE_LISTING, Listing::new(rows, 2));

    assert_eq!(finished.code, ExitCode::SUCCESS, "{}", finished.err);
    assert_eq!(finished.out, format!("NAME  STATE\napi   up\n  + {long}\n"));
    assert!(finished.err.contains("1 of 2"), "{}", finished.err);
}

/// Returns a batch of `tool show` that holds `rows`, one for each key.
fn answered<R: Row>(rows: Vec<R>, keys: &[&str]) -> Batch<R, Error> {
    let mut batch = Batch::new(&["show"], "names", keys);
    batch.extend(rows.into_iter().map(Ok));
    batch
}

#[test]
fn room_counts_each_row_s_child_lines_under_its_table_row() {
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    let batch = answered(api_and_web(), &["api", "web"]);
    let mut room = output.room(SERVICE_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, SERVICE_BATCH, batch);

    assert!(finished.out.contains("  - cache\n"), "{}", finished.out);
    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
}

#[test]
fn room_counts_each_row_s_child_lines_under_its_own_text_line() {
    const LINED_BATCH: Document<Batch<Lined, Error>> =
        Document::new("ops", "lined-batch", 1, "The test services.");
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list"],
    );
    let rows = vec![lined("api", &["database\t1"]), lined("web", &[])];
    let batch = answered(rows, &["api", "web"]);
    let mut room = output.room(LINED_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, LINED_BATCH, batch);

    assert_eq!(finished.out, "service api\n  database\\t1\nservice web\n");
    assert_eq!(room.printed(), finished.out.len());
}

#[test]
fn room_counts_the_children_field_in_compact_json() {
    let guide = Guide::new().output_budget(1 << 20, "the tests print little");
    let output = built_with(
        <Tool as clap::CommandFactory>::command(),
        &guide,
        &["tool", "list", "--json"],
    );
    let batch = answered(api_and_web(), &["api", "web"]);
    let mut room = output.room(SERVICE_BATCH);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, SERVICE_BATCH, batch);

    assert!(finished.out.contains("\"components\""), "{}", finished.out);
    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
}

#[test]
fn a_row_type_without_children_has_no_child_lines_by_default() {
    let row = super::mercury();

    assert_eq!(row.child_lines(), Vec::<String>::new());
    assert_eq!(<super::Planet as Row>::CHILDREN, None);
}
