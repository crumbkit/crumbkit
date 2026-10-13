//! Tests for the view flags `--fields`, `--long` and `--full` on a leaf
//! whose declared document cannot use them: its help leaves them out, and
//! `parse` refuses them before the handler runs.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::parse_to;
use crate::help::{Exit, Guide, Leaf, Rename};
use crate::twins::assert_one_word_apart;
use crate::{Bytes, Document, Output, Prose, Row, Sink, Spelling, Stream};

/// A row with a compact and a long column.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The number of pages.
    pages: u32,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["pages"];
}

/// A row without long columns.
#[derive(Serialize, JsonSchema)]
struct Id {
    /// The book's id.
    id: u32,
}

impl Row for Id {
    const COLUMNS: &'static [&'static str] = &["id"];
}

const BOOKS: Document<Vec<Book>> = Document::new("tool", "books", 1, "The books.");
const IDS: Document<Vec<Id>> = Document::new("tool", "ids", 1, "The ids.");
const COUNT: Document<Prose<String>> = Document::new("tool", "count", 1, "The count.");
const EXPORT: Document<Bytes> = Document::new("tool", "export", 1, "The export.");
const DUMP: Document<Vec<Book>> =
    Document::new("tool", "dump", 1, "The books as another tool reads them.").without_envelope();
const WATCH: Document<Stream<Id>> = Document::new("tool", "watch", 1, "The ids as they come.");

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).document(BOOKS),
    Leaf::new(&["books", "ids"]).document(IDS),
    Leaf::new(&["books", "count"]).document(COUNT),
    Leaf::new(&["books", "export"]).document(EXPORT),
    Leaf::new(&["books", "dump"]).document(DUMP),
    Leaf::new(&["books", "watch"]).stream(WATCH),
    Leaf::new(&["books", "plain"]),
    Leaf::new(&["books", "own"]).document(COUNT),
]);

/// A tool with crumbkit's output flags on its root and one group of leaves.
/// The leaf `own` declares its own `--long`, which is not global.
fn tool() -> Command {
    let leaves = ["list", "ids", "count", "export", "dump", "watch", "plain"]
        .map(|name| Command::new(name).about("A leaf"));
    let own = Command::new("own").about("A leaf with its own --long").arg(
        Arg::new("long")
            .long("long")
            .action(ArgAction::SetTrue)
            .help("Count the long books"),
    );
    Output::augment_args(Command::new("tool"))
        .subcommand_required(true)
        .subcommand(
            Command::new("books")
                .about("Work with books")
                .subcommand_required(true)
                .subcommands(leaves)
                .subcommand(own),
        )
}

/// What `parse_to` printed, and its exit code when it did not return a
/// parsed type.
struct Parsed {
    code: Option<ExitCode>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `tool()` with the guide's leaves and `args`.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &GUIDE,
        Spelling::COLONS,
        args,
    );
    Parsed {
        code: result.err().map(ExitCode::from),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Returns the `--help` page of the command at `path`.
fn long_help(path: &[&str]) -> String {
    let args: Vec<&str> = ["tool"]
        .iter()
        .chain(path)
        .chain(&["--help"])
        .copied()
        .collect();
    let parsed = parse(&args);
    assert_eq!(parsed.code, Some(ExitCode::SUCCESS), "{}", parsed.err);
    parsed.out
}

/// Returns the `Global flags:` line of the `-h` page of the command at
/// `path`.
fn global_flags_line(path: &[&str]) -> String {
    let args: Vec<&str> = ["tool"]
        .iter()
        .chain(path)
        .chain(&["-h"])
        .copied()
        .collect();
    let parsed = parse(&args);
    parsed
        .out
        .lines()
        .find(|line| line.starts_with("Global flags:"))
        .unwrap_or_default()
        .to_owned()
}

/// Returns which of `--fields`, `--long` and `--full` the `--help` page of
/// the command at `path` lists.
fn listed(path: &[&str]) -> [bool; 3] {
    let page = long_help(path);
    ["--fields <FIELD>", "--long\n", "--full\n"].map(|flag| page.contains(&format!("  {flag}")))
}

#[test]
fn the_help_of_a_prose_leaf_lists_no_fields_long_or_full_flag() {
    assert_eq!(listed(&["books", "count"]), [false, false, false]);
}

#[test]
fn the_help_of_a_bytes_leaf_lists_no_fields_long_or_full_flag() {
    assert_eq!(listed(&["books", "export"]), [false, false, false]);
}

#[test]
fn the_help_of_a_leaf_whose_rows_have_no_long_columns_lists_no_long_flag() {
    assert_eq!(listed(&["books", "ids"]), [true, false, true]);
}

#[test]
fn the_help_of_an_unwrapped_leaf_lists_no_full_flag() {
    assert_eq!(listed(&["books", "dump"]), [true, true, false]);
}

#[test]
fn the_help_of_a_stream_leaf_offers_the_flags_that_its_rows_can_use() {
    assert_eq!(listed(&["books", "watch"]), [true, false, true]);
}

#[test]
fn the_help_of_a_leaf_without_a_document_keeps_every_view_flag() {
    assert_eq!(listed(&["books", "plain"]), [true, true, true]);
    assert_eq!(listed(&["books", "list"]), [true, true, true]);
}

#[test]
fn the_help_of_a_branch_keeps_every_view_flag() {
    assert_eq!(listed(&["books"]), [true, true, true]);
}

#[test]
fn the_global_flags_line_of_a_prose_leaf_names_only_json_and_the_tool_s_own_flags() {
    assert_eq!(
        global_flags_line(&["books", "count"]),
        "Global flags: --json. `tool books count --help` describes them."
    );
    assert_eq!(
        global_flags_line(&["books", "list"]),
        "Global flags: --json, --fields <FIELD>, --long, --full. `tool books list --help` describes them."
    );
}

#[test]
fn long_on_a_prose_leaf_is_refused_before_the_handler_runs() {
    let refused = ["tool", "books", "count", "--long"];
    let admitted = ["tool", "books", "count"];
    assert_one_word_apart(&refused, &admitted);
    assert_eq!(parse(&admitted).code, None);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    assert_eq!(
        parsed.err,
        "\
error: --long adds the long columns of a table, and this command prints prose
  help:  use --json to read every field of the result
  code:  crumbkit::cli::usage
"
    );
}

#[test]
fn long_typed_before_the_command_path_is_refused_for_a_leaf_that_does_not_offer_it() {
    let parsed = parse(&["tool", "--long", "books", "ids"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(
        parsed.err,
        "\
error: --long adds the long columns of a table, and this command's rows have none
  help:  use --fields to name the columns to show
  code:  crumbkit::cli::usage
"
    );
}

#[test]
fn full_on_an_unwrapped_leaf_is_refused_with_a_hint_that_json_prints_every_field() {
    let parsed = parse(&["tool", "books", "dump", "--json", "--full"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    let document: serde_json::Value = serde_json::from_str(&parsed.out).unwrap();
    assert_eq!(
        document["message"],
        "--full prints every field of every row, and this command's JSON already holds every field"
    );
    assert_eq!(
        document["recovery"]["text"],
        "drop --full, because --json already prints the whole result"
    );
    assert_eq!(
        document["fields"],
        serde_json::json!({"argument": "--full"})
    );
}

#[test]
fn fields_on_a_bytes_leaf_is_refused_with_the_bytes_wording() {
    let parsed = parse(&["tool", "books", "export", "--fields", "id"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed.err.starts_with(
            "error: --fields selects the columns of a table, and this command prints raw bytes\n"
        ),
        "{}",
        parsed.err
    );
}

#[test]
fn a_tool_s_own_long_argument_on_a_leaf_is_neither_hidden_nor_refused() {
    let page = long_help(&["books", "own"]);
    let parsed = parse(&["tool", "books", "own", "--long"]);

    assert!(page.contains("Count the long books"), "{page}");
    assert_eq!(parsed.code, None, "{}", parsed.err);
}

#[test]
fn an_unoffered_view_flag_prints_the_tool_s_usage_code_and_exits_with_the_code_its_guide_lists() {
    const USAGE: crate::Code = crate::Code::new("tool", "cli", "usage");
    const RENAMED: Guide = Guide::new()
        .leaves(&[Leaf::new(&["books", "count"]).document(COUNT)])
        .codes(&[Rename::new(&crate::codes::CLI_USAGE, &USAGE)])
        .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])]);
    let mut out = Vec::new();
    let mut err = Vec::new();

    let stop = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        &RENAMED,
        Spelling::COLONS,
        ["tool", "books", "count", "--long"],
    )
    .err()
    .unwrap();

    assert_eq!(stop.code(), ExitCode::from(64));
    let err = String::from_utf8(err).unwrap();
    assert!(err.ends_with("  code:  tool::cli::usage\n"), "{err}");
}
