//! Tests for the view flags `--fields`, `--long` and `--full` on a leaf
//! whose declared document cannot use them: its help leaves them out, and
//! `parse` refuses them before the handler runs.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;

use super::parse_to;
use crate::Envelope;
use crate::help::{Exit, Guide, Leaf, Rename};
use crate::twins::assert_one_word_apart;
use crate::{Batch, Bytes, Document, Error, Headed, Prose, Row, Sink, Stream};

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

/// A row that holds a book's text, which a headed document prints under a
/// header.
#[derive(Serialize, JsonSchema)]
struct Page {
    /// The book's id.
    id: u32,
    /// The book's text.
    #[serde(flatten)]
    content: Bytes,
}

impl Row for Page {
    const COLUMNS: &'static [&'static str] = &["id"];
}

impl Headed for Page {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

const BOOKS: Document<Vec<Book>> = Document::new("tool", "books", 1, "The books.");
const IDS: Document<Vec<Id>> = Document::new("tool", "ids", 1, "The ids.");
const COUNT: Document<Prose<String>> = Document::new("tool", "count", 1, "The count.");
const EXPORT: Document<Bytes> = Document::new("tool", "export", 1, "The export.");
const DUMP: Document<Vec<Book>> =
    Document::new("tool", "dump", 1, "The books as another tool reads them.")
        .envelope(Envelope::Without);
const WATCH: Document<Stream<Id>> = Document::new("tool", "watch", 1, "The ids as they come.");
const READ: Document<Batch<Page, Error>> =
    Document::new("tool", "read", 1, "The text of each book.").headed();

/// A row that a tool's readers expect whole, with a compact and a long
/// column.
#[derive(Serialize, JsonSchema)]
struct Shelf {
    /// The shelf's id.
    id: u32,
    /// The number of books on the shelf.
    books: u32,
}

impl Row for Shelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LONG_COLUMNS: &'static [&'static str] = &["books"];
    const WHOLE: bool = true;
}

/// A row that serializes as an array of its values, so it has no columns.
#[derive(Serialize, JsonSchema)]
struct Answer(u32, u32);

impl Row for Answer {
    const COLUMNS: &'static [&'static str] = &[];
    const LINE: Option<&'static str> = Some("<LEFT> <RIGHT>");
    const WHOLE: bool = true;
}

const SHELVES: Document<Vec<Shelf>> = Document::new("tool", "shelves", 1, "The shelves.");
const QUERY: Document<Vec<Answer>> = Document::new("tool", "query", 1, "The answers.");

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).document(BOOKS),
    Leaf::new(&["books", "ids"]).document(IDS),
    Leaf::new(&["books", "count"]).document(COUNT),
    Leaf::new(&["books", "export"]).document(EXPORT),
    Leaf::new(&["books", "dump"]).document(DUMP),
    Leaf::new(&["books", "watch"]).stream(WATCH),
    Leaf::new(&["books", "read"]).document(READ),
    Leaf::new(&["books", "plain"]),
    Leaf::new(&["books", "own"]).document(COUNT),
    Leaf::new(&["books", "shelves"]).document(SHELVES),
    Leaf::new(&["books", "query"]).document(QUERY),
]);

/// A tool with crumbkit's output flags on its root and one group of leaves.
/// The leaf `own` declares its own `--long`, which is not global.
fn tool() -> Command {
    let leaves = [
        "list", "ids", "count", "export", "dump", "watch", "read", "plain", "shelves", "query",
    ]
    .map(|name| Command::new(name).about("A leaf"));
    let own = Command::new("own").about("A leaf with its own --long").arg(
        Arg::new("long")
            .long("long")
            .action(ArgAction::SetTrue)
            .help("Count the long books"),
    );
    crate::ViewFlags::augment_args(Command::new("tool"))
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
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), &GUIDE, args);
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
fn the_help_of_a_leaf_whose_rows_print_whole_lists_no_full_flag() {
    assert_eq!(listed(&["books", "shelves"]), [true, true, false]);
}

#[test]
fn the_help_of_a_leaf_whose_whole_rows_have_no_columns_lists_no_fields_long_or_full_flag() {
    assert_eq!(listed(&["books", "query"]), [false, false, false]);
}

#[test]
fn fields_on_a_leaf_whose_whole_rows_have_no_columns_is_refused_before_the_handler_runs() {
    let refused = ["tool", "books", "query", "--fields", "id"];
    let admitted = ["tool", "books", "query"];
    assert_eq!(parse(&admitted).code, None, "{}", parse(&admitted).err);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(
        parsed.err,
        "\
error: --fields selects the columns of a table, and this command's rows have no columns
  help:  use --json to read every field of each row
  code:  tool::cli::usage
"
    );
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
fn fields_without_json_on_a_headed_leaf_is_refused_before_the_handler_runs() {
    let refused = ["tool", "books", "read", "--fields", "id"];
    let admitted = ["tool", "books", "read", "--json", "--fields", "id"];
    assert_eq!(parse(&admitted).code, None, "{}", parse(&admitted).err);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed.err.starts_with(
            "error: --fields selects the fields of JSON rows, and this command's text prints each answer's bytes\n  help:  add --json, or drop --fields\n"
        ),
        "{}",
        parsed.err
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
  code:  tool::cli::usage
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
  code:  tool::cli::usage
"
    );
}

#[test]
fn full_with_json_on_an_unwrapped_leaf_is_accepted() {
    let parsed = parse(&["tool", "books", "dump", "--json", "--full"]);

    assert_eq!(parsed.code, None, "{}", parsed.out);
    assert_eq!(parsed.err, "");
}

#[test]
fn long_and_full_with_json_on_a_prose_leaf_are_accepted() {
    for flag in ["--long", "--full"] {
        let parsed = parse(&["tool", "books", "count", "--json", flag]);

        assert_eq!(parsed.code, None, "{flag}: {}", parsed.out);
        assert_eq!(parsed.err, "", "{flag}");
    }
}

#[test]
fn long_and_full_with_json_on_a_headed_leaf_are_accepted() {
    for flag in ["--long", "--full"] {
        let parsed = parse(&["tool", "books", "read", "--json", flag]);

        assert_eq!(parsed.code, None, "{flag}: {}", parsed.out);
        assert_eq!(parsed.err, "", "{flag}");
    }
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
        ["tool", "books", "count", "--long"],
    )
    .err()
    .unwrap();

    assert_eq!(stop.exit_code(), ExitCode::from(64));
    let err = String::from_utf8(err).unwrap();
    assert!(err.ends_with("  code:  tool::cli::usage\n"), "{err}");
}

#[test]
fn full_without_json_is_refused_in_crumbkit_s_words_before_the_handler_runs() {
    let refused = ["tool", "books", "list", "--full"];
    let admitted = ["tool", "books", "list", "--json", "--full"];
    assert_one_word_apart(&refused, &admitted);
    assert_eq!(parse(&admitted).code, None, "{}", parse(&admitted).err);

    let parsed = parse(&refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    assert_eq!(
        parsed.err,
        "\
error: --full prints every field of every row in JSON, so it needs --json
  help:  add --json, or drop --full
  code:  tool::cli::usage
"
    );
}

#[test]
fn fields_with_long_is_refused_in_crumbkit_s_words_on_a_command_that_the_guide_does_not_name() {
    let run = |args: &[&str]| {
        let mut err = Vec::new();
        let result = parse_to::<(), _, _, _, _>(
            &mut Sink::new(Vec::new(), &mut err),
            tool(),
            &Guide::new(),
            args,
        );
        (
            result.err().map(ExitCode::from),
            String::from_utf8(err).unwrap(),
        )
    };
    let refused = ["tool", "books", "list", "--fields", "id", "--long"];
    let admitted = ["tool", "books", "list", "--fields", "id"];
    assert_one_word_apart(&refused, &admitted);
    assert_eq!(run(&admitted).0, None, "{}", run(&admitted).1);

    let (code, err) = run(&refused);

    assert_eq!(code, Some(ExitCode::from(2)));
    assert_eq!(
        err,
        "\
error: --fields prints only the fields it names, so it cannot be used with --long
  help:  drop --long, and name the long columns in --fields
  code:  tool::cli::usage
"
    );
}
