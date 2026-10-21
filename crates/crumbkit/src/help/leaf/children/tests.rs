//! Tests for the `Children:` line of a leaf whose rows print their children
//! as lines under them (ADR-0120, ADR-0137).

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

/// A workspace whose applications print under it, folded into one line
/// when every application is stopped, which its help says.
#[derive(Serialize, JsonSchema)]
struct Workspace {
    /// The workspace's name.
    name: String,
    /// The applications that the workspace runs.
    applications: Vec<String>,
}

impl Row for Workspace {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("applications");
    const CHILDREN_HELP: Option<&'static str> = Some(
        "one line for each application, or one folded line when every application is stopped.",
    );
}

/// A service whose components print at column 1 under it.
#[derive(Serialize, JsonSchema)]
struct Flush {
    /// The service's name.
    name: String,
    /// The parts that the service runs on.
    components: Vec<String>,
}

impl Row for Flush {
    const COLUMNS: &'static [&'static str] = &["name"];
    const CHILDREN: Option<&'static str> = Some("components");
    const CHILD_INDENT: u8 = 0;
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

const WORKSPACES: Document<Vec<Workspace>> =
    Document::new("tool", "services-list", 1, "The workspaces.");
const FLUSH: Document<Vec<Flush>> = Document::new("tool", "services-list", 1, "The services.");

const WORKSPACES_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every workspace")])
    .effects(Effects::read_only())
    .document(WORKSPACES)];
const FLUSH_LEAF: &[Leaf] = &[Leaf::new(&["services", "list"])
    .examples(&[Example::new(&[], "List every service")])
    .effects(Effects::read_only())
    .document(FLUSH)];
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
fn the_children_line_prints_its_text_after_the_label() {
    assert_eq!(
        format!(
            "{}\n",
            children_line("one indented line under each row for each of its components.")
        ),
        CHILDREN
    );
}

#[test]
fn a_row_type_that_sets_no_text_and_the_default_indent_prints_the_existing_sentence() {
    let short = help(SERVICES_LEAF, &["tool", "services", "list", "-h"]);
    let long = help(SERVICES_LEAF, &["tool", "services", "list", "--help"]);

    assert!(short.contains(CHILDREN), "{short}");
    assert!(long.contains(CHILDREN), "{long}");
}

#[test]
fn a_row_type_that_sets_the_help_text_prints_it_on_the_children_line_of_short_and_long_help() {
    let short = help(WORKSPACES_LEAF, &["tool", "services", "list", "-h"]);
    let long = help(WORKSPACES_LEAF, &["tool", "services", "list", "--help"]);

    let expected = "Children: one line for each application, or one folded line when every application is stopped.\n";
    assert!(short.contains(expected), "{short}");
    assert!(long.contains(expected), "{long}");
}

#[test]
fn a_row_type_with_an_indent_of_0_and_no_text_prints_a_sentence_that_does_not_say_indented() {
    let page = help(FLUSH_LEAF, &["tool", "services", "list", "--help"]);

    assert!(
        page.contains("Children: one line under each row for each of its components.\n"),
        "{page}"
    );
    assert!(!page.contains("indented"), "{page}");
}

#[test]
fn a_leaf_holds_the_help_text_of_its_row_type_and_the_text_that_help_prints() {
    assert_eq!(
        WORKSPACES_LEAF[0].children_help(),
        Some(
            "one line for each application, or one folded line when every application is stopped."
        )
    );
    assert_eq!(
        WORKSPACES_LEAF[0].children_text().as_deref(),
        WORKSPACES_LEAF[0].children_help()
    );
    assert_eq!(SERVICES_LEAF[0].children_help(), None);
    assert_eq!(
        SERVICES_LEAF[0].children_text().as_deref(),
        Some("one indented line under each row for each of its components.")
    );
    assert_eq!(Leaf::new(&["services", "list"]).children_text(), None);
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
