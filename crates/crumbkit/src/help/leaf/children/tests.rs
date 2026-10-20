//! Tests for the `Children:` line of a leaf whose rows print their children
//! as indented lines under them (ADR-0120).

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};
use schemars::JsonSchema;
use serde::Serialize;

use crate::help::{Effects, Example, Guide, Leaf, STREAM_LINE, children_line};
use crate::{Batch, Bytes, Document, Error, Headed, Row, Sink, Stream, parse_to};

/// A tool with a global `--json` and one leaf, `tool services list`.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("services").subcommand(Command::new("list").about("List the services")),
        )
}

/// Runs `parse_to` over `tool()` with `leaves` for `args`, which ask for
/// help, and returns the page it printed.
fn help(leaves: &'static [Leaf], args: &[&str]) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let guide = Guide::new().leaves(leaves);
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), &guide, args);
    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    String::from_utf8(out).unwrap()
}

/// A service whose components print under it.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: String,
    /// The parts that the service runs on.
    components: Vec<String>,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("components");
}

/// A service that prints its own text line, with its components under it.
#[derive(Serialize, JsonSchema)]
struct Lined {
    /// The service's name.
    name: String,
    /// The parts that the service runs on.
    components: Vec<String>,
}

impl Row for Lined {
    const COLUMNS: &'static [&'static str] = &["name"];
    const LINE: Option<&'static str> = Some("<NAME>");
    const CHILDREN: Option<&'static str> = Some("components");
}

/// A service's log file, which a headed batch prints under a header.
#[derive(Serialize, JsonSchema)]
struct Log {
    /// The service's name.
    name: String,
    /// The parts that the service runs on.
    components: Vec<String>,
    #[serde(flatten)]
    content: Bytes,
}

impl Row for Log {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("components");
}

impl Headed for Log {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

const SERVICES: Document<Vec<Service>> = Document::new("tool", "services-list", 1, "The services.");
const SERVICE_STREAM: Document<Stream<Service>> =
    Document::new("tool", "services-list", 1, "Each service as it is found.");
const LINED: Document<Vec<Lined>> = Document::new("tool", "services-list", 1, "The services.");
const LOGS: Document<Batch<Log, Error>> =
    Document::new("tool", "services-list", 1, "The services' logs.").headed();

const SERVICES_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every service")])
    .effects(Effects::read_only())
    .document(SERVICES)];
const STREAM_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every service")])
    .effects(Effects::read_only())
    .stream(SERVICE_STREAM)];
const LINED_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every service")])
    .effects(Effects::read_only())
    .document(LINED)];
const LOGS_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every service")])
    .effects(Effects::read_only())
    .document(LOGS)];

const CHILDREN: &str = "Children: one indented line under each row for each of its components.\n";

#[test]
fn a_leaf_holds_the_children_field_of_its_row_type() {
    assert_eq!(SERVICES_LEAF[0].children(), Some("components"));
    assert_eq!(Leaf::new(&["services", "list"]).children(), None);
}

#[test]
fn the_children_field_reads_as_a_column_with_its_property_and_description() {
    let column = SERVICES_LEAF[0].children_column().unwrap();

    assert_eq!(column.name(), "components");
    assert!(column.is_property(), "{column:?}");
    assert_eq!(
        column.description(),
        Some("The parts that the service runs on.")
    );
}

#[test]
fn the_children_line_names_the_field() {
    assert_eq!(format!("{}\n", children_line("components")), CHILDREN);
}

#[test]
fn the_help_of_a_leaf_whose_rows_print_children_names_the_field_after_the_effects() {
    let short = help(SERVICES_LEAF, &["tool", "services", "list", "-h"]);
    let long = help(SERVICES_LEAF, &["tool", "services", "list", "--help"]);

    let expected = format!("Effects: read-only, closed-world.\n{CHILDREN}");
    assert!(short.contains(&expected), "{short}");
    assert!(long.contains(&expected), "{long}");
}

#[test]
fn the_children_line_of_a_stream_follows_its_output_line() {
    let page = help(STREAM_LEAF, &["tool", "services", "list", "-h"]);

    assert!(
        page.contains(&format!("{STREAM_LINE}\n{CHILDREN}")),
        "{page}"
    );
}

#[test]
fn the_children_line_follows_the_text_line() {
    let page = help(LINED_LEAF, &["tool", "services", "list", "-h"]);

    let expected = format!("Text: one line per row, <NAME>; --fields prints a table.\n{CHILDREN}");
    assert!(page.contains(&expected), "{page}");
}

#[test]
fn a_headed_leaf_has_no_children_line() {
    let page = help(LOGS_LEAF, &["tool", "services", "list", "--help"]);

    assert!(!page.contains("Children:"), "{page}");
}

#[test]
fn the_help_of_a_leaf_whose_rows_name_no_children_has_no_children_line() {
    const PLAIN: &[Leaf] = &[Leaf::new(&["services", "list"])
        .examples(&[Example::new(&[], "List every service")])
        .effects(Effects::read_only())];
    let page = help(PLAIN, &["tool", "services", "list", "--help"]);

    assert!(!page.contains("Children:"), "{page}");
}
