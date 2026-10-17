//! Tests for `commands list`, which exports one branch of a tool's command tree.

use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgGroup, Args as _, Command};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};

use super::{COMMANDS_LIST, CommandRow, CommandsList, list};
use crate::help::{Example, Guide, Leaf};
use crate::{Document, Error, Listing, Output, Row, Sink, Spelling, finish_to};

mod arguments;
mod exits;

/// A row type for the leaf whose columns the export lists.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's id.
    id: u32,
    /// The book's title.
    title: String,
    /// The year the book was first published.
    ///
    /// The year is the one on the title page.
    year: u16,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["year"];
}

const BOOKS_LIST: Document<Vec<Book>> = Document::new("shelf", "books-list", 1, "The test rows.");

/// A guide with one leaf, whose example has a word that needs quoting.
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
    .examples(&[
        Example::new(&[], "Print the first page of books"),
        Example::new(&["--form", "short stories"], "Print the collections"),
    ])
    .document(BOOKS_LIST)]);

/// A tool with a global flag, a visible branch, a hidden branch, a hidden
/// argument and the `commands list` command, as `crumbkit::parse` returns
/// it with clap's generated `help` subcommand turned off.
fn tool() -> Command {
    Command::new("shelf")
        .about("Browse a catalogue")
        .version("1.0.0")
        .disable_help_subcommand(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("books")
                .about("Work with the books")
                .subcommand(
                    Command::new("list")
                        .about("List the books")
                        .arg(
                            Arg::new("limit")
                                .long("limit")
                                .short('n')
                                .value_name("N")
                                .default_value("4")
                                .help("Print at most N books")
                                .long_help("Print at most N books.\n\nThe default is 4."),
                        )
                        .arg(
                            Arg::new("form")
                                .long("form")
                                .action(ArgAction::Append)
                                .value_delimiter(',')
                                .value_parser(["novel", "short stories"]),
                        )
                        .arg(
                            Arg::new("trace")
                                .long("trace")
                                .hide(true)
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show books")
                        .arg(Arg::new("ids").value_name("ID").num_args(1..=100))
                        .arg(Arg::new("from").long("from").value_name("FILE"))
                        .group(ArgGroup::new("source").args(["ids", "from"]).required(true)),
                )
                .subcommand(
                    Command::new("purge")
                        .about("Remove every book")
                        .hide(true)
                        .subcommand(Command::new("all")),
                ),
        )
        .subcommand(
            Command::new("commands")
                .about("Explore the commands")
                .subcommand(CommandsList::augment_args(
                    Command::new("list").about("List a branch of commands"),
                )),
        )
}

/// Returns the output flags with the tool's command set, as `main` sets it.
fn output() -> Output {
    let mut output = Output::default();
    output.set_command(tool());
    output
}

/// Parses `words` after `shelf commands list` into the command's arguments.
fn args(words: &[&str]) -> CommandsList {
    let mut command = tool();
    let matches = command
        .try_get_matches_from_mut(["shelf", "commands", "list"].iter().chain(words))
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    <CommandsList as clap::FromArgMatches>::from_arg_matches(matches).unwrap()
}

/// Lists the branch that `words` name and returns the listing as JSON.
fn listed(words: &[&str]) -> Value {
    serde_json::to_value(list(&mut output(), &GUIDE, &args(words)).unwrap()).unwrap()
}

/// Returns the path of each row of `rows`.
fn paths(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap())
        .collect()
}

/// Returns the row whose path is `path`.
fn row<'a>(rows: &'a Value, path: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == path)
        .unwrap()
}

/// Returns the names of the arguments of the row whose path is `path`.
fn argument_names<'a>(rows: &'a Value, path: &str) -> Vec<&'a str> {
    row(rows, path)["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument["name"].as_str().unwrap())
        .collect()
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Lists the branch that `words` name with `output`, and finishes the
/// result into memory as a tool's handler does.
fn finish(output: &mut Output, words: &[&str]) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Listing<CommandRow>, Error> = list(output, &GUIDE, &args(words));
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        COMMANDS_LIST,
        result,
    );
    Finished {
        code: ExitCode::from(code),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn a_path_lists_its_command_and_then_each_visible_command_below_it_depth_first() {
    let rows = listed(&["books"]);

    assert_eq!(
        paths(&rows),
        ["shelf books", "shelf books list", "shelf books show"]
    );
}

#[test]
fn no_path_lists_the_whole_tree_and_leaves_out_a_hidden_command_and_everything_below_it() {
    let rows = listed(&[]);

    assert_eq!(
        paths(&rows),
        [
            "shelf",
            "shelf books",
            "shelf books list",
            "shelf books show",
            "shelf commands",
            "shelf commands list",
        ]
    );
}

#[test]
fn a_path_resolves_an_alias_to_the_command_s_real_name() {
    let mut output = Output::default();
    output.set_command(tool().mut_subcommand("books", |books| books.alias("book")));
    let listing = list(&mut output, &GUIDE, &args(&["book", "show"])).unwrap();
    let rows = serde_json::to_value(listing).unwrap();

    assert_eq!(paths(&rows), ["shelf books show"]);
}

#[test]
fn a_row_leaves_out_hidden_arguments_and_the_help_and_version_flags() {
    let rows = listed(&[]);

    assert_eq!(argument_names(&rows, "shelf"), ["--json"]);
    assert_eq!(
        argument_names(&rows, "shelf books list"),
        ["--limit", "--form"]
    );
}

#[test]
fn a_global_argument_is_listed_once_on_the_command_that_defines_it() {
    let rows = listed(&[]);

    let listed_json: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|argument| argument["name"] == "--json")
        })
        .map(|row| row["path"].as_str().unwrap())
        .collect();
    assert_eq!(listed_json, ["shelf"]);
    assert_eq!(row(&rows, "shelf")["arguments"][0]["global"], true);
}

#[test]
fn a_branch_lists_the_global_arguments_of_the_commands_above_it_once_on_its_first_row() {
    let rows = listed(&["books"]);

    assert_eq!(argument_names(&rows, "shelf books"), ["--json"]);
    assert_eq!(row(&rows, "shelf books")["arguments"][0]["global"], true);
    assert_eq!(
        argument_names(&rows, "shelf books list"),
        ["--limit", "--form"]
    );
    assert_eq!(
        argument_names(&rows, "shelf books show"),
        ["<ID>", "--from"]
    );
}

#[test]
fn an_argument_holds_what_its_help_shows() {
    let rows = listed(&["books"]);

    assert_eq!(
        row(&rows, "shelf books list")["arguments"],
        json!([
            {
                "name": "--limit",
                "short": "-n",
                "value_names": ["N"],
                "required": false,
                "many": false,
                "defaults": ["4"],
                "possible_values": [],
                "help": "Print at most N books",
                "long_help": "Print at most N books.\n\nThe default is 4.",
                "global": false,
                "one_of": [],
            },
            {
                "name": "--form",
                "short": null,
                "value_names": ["form"],
                "required": false,
                "many": true,
                "defaults": [],
                "possible_values": ["novel", "short stories"],
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": [],
            },
        ])
    );
    assert_eq!(
        row(&rows, "shelf books show")["arguments"],
        json!([
            {
                "name": "<ID>",
                "short": null,
                "value_names": ["ID"],
                "required": false,
                "many": true,
                "defaults": [],
                "possible_values": [],
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": ["--from"],
            },
            {
                "name": "--from",
                "short": null,
                "value_names": ["FILE"],
                "required": false,
                "many": false,
                "defaults": [],
                "possible_values": [],
                "help": null,
                "long_help": null,
                "global": false,
                "one_of": ["<ID>"],
            },
        ])
    );
}

#[test]
fn one_of_names_visible_required_group_members_and_conflicts_of_required_arguments() {
    let command = Command::new("show")
        .arg(Arg::new("ids").value_name("ID").num_args(1..=100))
        .arg(Arg::new("from").long("from"))
        .arg(Arg::new("stdin").long("stdin").hide(true))
        .arg(Arg::new("quiet").long("quiet").conflicts_with("from"))
        .arg(
            Arg::new("all")
                .long("all")
                .required(true)
                .conflicts_with_all(["quiet", "stdin"]),
        )
        .group(
            ArgGroup::new("source")
                .args(["ids", "from", "stdin"])
                .required(true),
        )
        .group(ArgGroup::new("loose").args(["ids", "quiet"]));
    let row = CommandRow::new("shelf", &["show"], &command, None, &Guide::new());
    let row = serde_json::to_value(row).unwrap();

    let one_of: Value = row["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument["one_of"].clone())
        .collect();
    assert_eq!(one_of, json!([["--from"], ["<ID>"], [], ["--quiet"]]));
}

#[test]
fn a_row_holds_its_leaf_s_examples_as_quoted_command_lines_and_its_columns() {
    let rows = listed(&["books", "list"]);

    assert_eq!(
        rows,
        json!([{
            "path": "shelf books list",
            "about": "List the books",
            "long_about": null,
            "arguments": row(&rows, "shelf books list")["arguments"],
            "examples": [
                {
                    "command": "shelf books list",
                    "about": "Print the first page of books",
                },
                {
                    "command": "shelf books list --form 'short stories'",
                    "about": "Print the collections",
                },
            ],
            "columns": [
                { "name": "id", "long": false, "description": "The book's id." },
                { "name": "title", "long": false, "description": "The book's title." },
                {
                    "name": "year",
                    "long": true,
                    "description": "The year the book was first published.",
                },
            ],
            "form": "document",
            "line": null,
            "exits": [],
            "effects": null,
            "writes": null,
        }])
    );
}

#[test]
fn a_command_without_a_leaf_has_no_examples_and_no_columns() {
    let rows = listed(&["books", "show"]);

    assert_eq!(rows[0]["examples"], json!([]));
    assert_eq!(rows[0]["columns"], json!([]));
}

#[test]
fn the_rows_are_the_same_from_the_command_that_parse_returns_after_it_added_help_lines() {
    let words = ["shelf", "books", "list"];
    let ((), parsed) = crate::parse_from::<(), _, _>(tool(), &GUIDE, Spelling::COLONS, words)
        .ok()
        .unwrap();
    let mut from_parse = Output::default();
    from_parse.set_command(parsed);

    let rows = list(&mut from_parse, &GUIDE, &args(&[])).unwrap();

    assert_eq!(rows, list(&mut output(), &GUIDE, &args(&[])).unwrap());
}

#[test]
fn the_limit_keeps_the_first_rows_and_the_total_counts_the_whole_branch() {
    let finished = finish(&mut output(), &["--limit", "2"]);

    let expected = "\
PATH         ABOUT
shelf        Browse a catalogue
shelf books  Work with the books
";
    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, expected);
    assert_eq!(
        finished.err,
        "Showing 2 of 6 commands. --all shows every one.\n\
         Exit codes: 0, 1 and 2 are crumbkit's. `shelf help exit-codes` describes them.\n"
    );
}

#[test]
fn a_path_that_names_no_command_fails_and_its_recovery_lists_the_longest_visible_prefix() {
    let finished = finish(&mut output(), &["books", "shw", "all"]);

    let expected = "\
error: `shelf books shw` names no visible command
  help:  run `shelf commands list books`
  code:  crumbkit::commands::not_found
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.err, expected);
}

#[test]
fn a_path_that_names_a_hidden_command_fails_as_one_that_names_no_command() {
    let finished = finish(&mut output(), &["books", "purge", "all"]);

    let expected = "\
error: `shelf books purge` names no visible command
  help:  run `shelf commands list books`
  code:  crumbkit::commands::not_found
";
    assert_eq!(finished.err, expected);
}

#[test]
fn a_path_whose_first_word_names_no_command_recovers_with_the_whole_tree() {
    let finished = finish(&mut output(), &["comets"]);

    assert!(finished.err.contains("help:  run `shelf commands list`\n"));
}

#[test]
fn the_recovery_has_no_next_step_when_the_runtime_tree_hides_commands_list() {
    let mut output = Output::default();
    output.set_command(tool().mut_subcommand("commands", |commands| commands.hide(true)));

    let finished = finish(&mut output, &["comets"]);

    assert!(
        finished
            .err
            .contains("help:  no next step: the suggested command is not available\n")
    );
}

#[test]
fn without_a_command_set_the_export_fails_with_no_next_step() {
    let finished = finish(&mut Output::default(), &[]);

    let expected = "\
error: the tool has not kept its command tree, so it cannot list its commands
  help:  no next step: the tool's main does not pass its command to Output::set_command
  code:  crumbkit::commands::unavailable
";
    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(finished.err.ends_with(expected), "{}", finished.err);
}

#[test]
fn the_row_schema_lists_and_requires_every_field_that_a_row_serializes() {
    let rows = listed(&["books", "list"]);
    let schema = schemars::schema_for!(CommandRow);

    assert_fields_match(&rows[0], schema.as_value());
    assert_fields_match(
        &rows[0]["arguments"][0],
        &schema.as_value()["properties"]["arguments"]["items"],
    );
    assert_fields_match(
        &rows[0]["examples"][0],
        &schema.as_value()["properties"]["examples"]["items"],
    );
    assert_fields_match(
        &rows[0]["columns"][0],
        &schema.as_value()["properties"]["columns"]["items"],
    );
}

/// Asserts that the keys of the object `value` are exactly the properties
/// of the object schema `schema`, and that the schema requires each.
fn assert_fields_match(value: &Value, schema: &Value) {
    let mut fields: Vec<&String> = value.as_object().unwrap().keys().collect();
    fields.sort();
    let mut properties: Vec<&String> = schema["properties"].as_object().unwrap().keys().collect();
    properties.sort();
    let mut required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    required.sort_unstable();
    assert_eq!(fields, properties);
    assert_eq!(fields, required);
}

#[test]
fn the_checked_in_schema_matches_the_commands_document_schema() {
    let generated = crate::schemas::document_schema(COMMANDS_LIST);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";

    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/commands-list-v1.json"
    ));
    assert_eq!(
        checked_in, generated,
        "schemas/commands-list-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}

#[test]
fn the_schema_describes_path_with_the_text_of_its_help() {
    let schema = schemars::schema_for!(CommandsList).to_value();
    let command = CommandsList::augment_args(Command::new("list"));
    let path = command
        .get_arguments()
        .find(|arg| arg.get_id() == "path")
        .unwrap();

    assert_eq!(
        schema["properties"]["path"]["description"],
        path.get_help().unwrap().to_string()
    );
}
