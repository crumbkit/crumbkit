//! Tests for `aliases list`, which lists the built-in and personal aliases.

use std::path::Path;
use std::process::ExitCode;

use crumbkit::{Error, Listing, Output, Sink, Spelling, finish_to};
use serde_json::{Value, json};

use super::{ALIASES_LIST, AliasRow, AliasesList, list, listing};
use crate::testing::tool;

/// The path that the personal rows name as their source.
const FILE: &str = "/home/ada/.config/shelf/aliases.toml";

/// Returns the output of a run of the test tool with every view flag off.
fn output() -> Output {
    Output::new(
        crumbkit::ViewFlags::new(),
        tool(),
        &crumbkit::help::Guide::new(),
        ["shelf"],
    )
}

/// Parses `words` after `shelf aliases list` into the command's arguments.
fn args(words: &[&str]) -> AliasesList {
    let mut command = tool();
    let matches = command
        .try_get_matches_from_mut(["shelf", "aliases", "list"].iter().chain(words))
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    <AliasesList as clap::FromArgMatches>::from_arg_matches(matches).unwrap()
}

/// Returns the rows for an aliases file at [`FILE`] that holds `text`, as
/// JSON.
fn listed(text: &str) -> Value {
    let listing = listing(&tool(), Some(Path::new(FILE)), Some(text), &args(&[]));
    serde_json::to_value(listing.unwrap()).unwrap()
}

/// Returns the row named `name`.
fn row<'a>(rows: &'a Value, name: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

/// What `finish_to` returned and wrote.
struct Finished {
    code: ExitCode,
    out: String,
    err: String,
}

/// Finishes `result` into memory as a tool's handler does.
fn finish(output: &mut Output, result: Result<Listing<AliasRow>, Error>) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        ALIASES_LIST,
        result,
    );
    Finished {
        code: ExitCode::from(code),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn built_in_aliases_at_every_level_come_first_and_name_their_full_path() {
    let rows = listed(r#"mine = ["books", "list"]"#);

    assert_eq!(
        rows,
        json!([
            { "name": "b", "command": "books", "problem": "", "from": "shelf" },
            { "name": "books ls", "command": "books list", "problem": "", "from": "shelf" },
            { "name": "mine", "command": "books list", "problem": "", "from": FILE },
        ])
    );
}

#[test]
fn the_aliases_of_a_hidden_command_and_of_everything_below_it_are_left_out() {
    let rows = listed("");

    let names: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["b", "books ls"]);
}

#[test]
fn personal_aliases_follow_in_name_order_with_their_words_quoted_for_a_shell() {
    let rows = listed(
        r#"
        zed = ["books", "show", "1"]
        austen = ["books", "list", "--author", "Jane Austen"]
        "#,
    );

    assert_eq!(rows[2]["name"], "austen");
    assert_eq!(rows[2]["command"], "books list --author 'Jane Austen'");
    assert_eq!(rows[3]["name"], "zed");
}

#[test]
fn aliases_with_a_problem_are_listed_before_healthy_aliases() {
    let rows = listed(
        r#"
        zed = ["books", "show", "1"]
        books = ["books", "show", "1"]
        austen = ["books", "list"]
        bad = 3
        "#,
    );

    let names: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["bad", "books", "b", "books ls", "austen", "zed"]);
}

#[test]
fn an_entry_named_after_a_command_or_built_in_alias_says_the_real_one_wins() {
    let rows = listed(
        r#"
        books = ["books", "show", "1"]
        adm = ["books", "list"]
        b = ["books", "list"]
        "#,
    );

    for name in ["books", "adm", "b"] {
        let personal = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == name && row["from"] == FILE)
            .unwrap();
        assert_eq!(
            personal["problem"],
            "the tool has a command or built-in alias of this name, which wins",
            "{name}"
        );
    }
}

#[test]
fn an_entry_named_help_says_that_the_tool_answers_help_with_its_own_help() {
    let rows = listed(
        r#"
        help = ["books", "list"]
        "#,
    );

    let personal = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "help" && row["from"] == FILE)
        .unwrap();
    assert_eq!(
        personal["problem"],
        "the tool answers the word help with its own help, which wins"
    );
}

#[test]
fn an_entry_whose_value_is_not_a_list_of_one_or_more_words_says_so() {
    let rows = listed(
        r#"
        empty = []
        line = "books list"
        mixed = ["books", 4]
        "#,
    );

    for name in ["empty", "line", "mixed"] {
        assert_eq!(
            row(&rows, name)["problem"],
            "the value must be a list of one or more words",
            "{name}"
        );
        assert_eq!(row(&rows, name)["command"], "", "{name}");
    }
}

#[test]
fn a_missing_file_lists_the_built_in_aliases_and_a_note_says_it_does_not_exist_yet() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("no-such-aliases.toml");

    let listing = list(&mut output(), Some(&missing), &args(&["--limit", "1"])).unwrap();
    let finished = finish(&mut output(), Ok(listing));

    assert_eq!(
        finished.out,
        "NAME  COMMAND  PROBLEM  FROM\nb     books             shelf\n"
    );
    assert_eq!(
        finished.err,
        format!(
            "Showing 1 of 2 aliases. --all shows every one.\n\
             Aliases file: {}, which does not exist yet\n",
            missing.display()
        )
    );
}

#[test]
fn a_note_on_standard_error_names_a_file_that_exists_even_when_it_is_empty() {
    let listing = listing(&tool(), Some(Path::new(FILE)), Some(""), &args(&[]));

    let finished = finish(&mut output(), listing.map_err(Error::from));

    assert!(
        finished
            .out
            .ends_with("books ls  books list           shelf\n"),
        "{}",
        finished.out
    );
    assert_eq!(finished.err, format!("Aliases file: {FILE}\n"));
}

#[test]
fn a_page_cut_by_a_limit_counts_every_alias_with_a_problem_in_its_note() {
    let text = "bad = 3\nbooks = [\"books\", \"show\", \"1\"]\n";
    let listing = listing(
        &tool(),
        Some(Path::new(FILE)),
        Some(text),
        &args(&["--limit", "1"]),
    );

    let finished = finish(&mut output(), listing.map_err(Error::from));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(
        finished.err,
        format!(
            "Showing 1 of 4 aliases. --all shows every one.\n\
             note: 2 of the aliases report a problem, so the command exits with 1.\n\
             Aliases file: {FILE}\n"
        )
    );
}

#[test]
fn a_note_says_why_a_tool_without_a_config_directory_has_no_aliases_file() {
    let listing = listing(&tool(), None, None, &args(&[]));

    let finished = finish(&mut output(), listing.map_err(Error::from));

    assert_eq!(
        finished.err,
        "shelf has no aliases file, because the home directory cannot be found.\n"
    );
}

#[test]
fn a_file_that_cannot_be_read_fails_with_its_path_and_the_reason() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"));

    let finished = finish(
        &mut output(),
        list(&mut output(), Some(directory), &args(&[])),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "");
    assert!(
        finished.err.starts_with(&format!(
            "error: cannot read the aliases file {}: ",
            directory.display()
        )),
        "{}",
        finished.err
    );
    assert!(
        finished
            .err
            .contains(&format!("help:  fix or remove {}", directory.display())),
        "{}",
        finished.err
    );
    assert!(
        finished
            .err
            .ends_with("code:  crumbkit::aliases::unreadable\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_file_that_cannot_be_parsed_fails_with_the_line_and_the_parser_s_message() {
    let error = listing(
        &tool(),
        Some(Path::new(FILE)),
        Some("mine = [\"books\"]\nours = ["),
        &args(&[]),
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .starts_with(&format!("cannot read the aliases file {FILE}: line 2: ")),
        "{error}"
    );
}

#[test]
fn the_checked_in_schema_matches_the_aliases_document_schema() {
    let generated = serde_json::to_string_pretty(&crumbkit::schemas::document_schema(ALIASES_LIST))
        .unwrap()
        + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/aliases-list-v1.json"
    ));

    assert_eq!(generated, checked_in);
}

#[test]
fn a_subcommand_without_a_doc_comment_takes_no_description_from_the_list_arguments() {
    let command = <AliasesList as clap::Args>::augment_args(clap::Command::new("list"));

    assert_eq!(command.get_about(), None);
    assert_eq!(command.get_long_about(), None);
}
