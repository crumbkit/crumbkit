//! Tests for `aliases list`, which lists the built-in and personal aliases.

use std::borrow::Cow;
use std::path::Path;
use std::process::ExitCode;

use crumbkit::{Error, Listing, Output, Sink, Spelling, finish_to};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema, schema_for};
use serde_json::{Value, json};

use super::{ALIASES, AliasRow, AliasesList, list, listing};
use crate::testing::tool;

/// The path that the personal rows name as their source.
const FILE: &str = "/home/ada/.config/shelf/aliases.toml";

/// Returns the output flags with the test tool's command set, as `main`
/// sets it.
fn output() -> Output {
    let mut output = Output::default();
    output.set_command(tool());
    output
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
fn finish(output: &Output, result: Result<Listing<AliasRow>, Error>) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        ALIASES,
        result,
    );
    Finished {
        code,
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
fn an_entry_named_after_a_command_or_built_in_alias_or_help_says_the_real_one_wins() {
    let rows = listed(
        r#"
        books = ["books", "show", "1"]
        adm = ["books", "list"]
        b = ["books", "list"]
        help = ["books", "list"]
        "#,
    );

    for name in ["books", "adm", "b", "help"] {
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

    let listing = list(&output(), Some(&missing), &args(&["--limit", "1"])).unwrap();
    let finished = finish(&output(), Ok(listing));

    assert_eq!(
        finished.out,
        format!(
            "NAME  COMMAND  PROBLEM  FROM\n\
             b     books             shelf\n\
             Showing 1 of 2 aliases. --all shows every one.\n\
             Aliases file: {}, which does not exist yet\n",
            missing.display()
        )
    );
}

#[test]
fn a_note_after_the_table_names_a_file_that_exists_even_when_it_is_empty() {
    let listing = listing(&tool(), Some(Path::new(FILE)), Some(""), &args(&[]));

    let finished = finish(&output(), listing.map_err(Error::from));

    assert!(
        finished.out.ends_with(&format!(
            "books ls  books list           shelf\nAliases file: {FILE}\n"
        )),
        "{}",
        finished.out
    );
}

#[test]
fn a_note_says_why_a_tool_without_a_config_directory_has_no_aliases_file() {
    let listing = listing(&tool(), None, None, &args(&[]));

    let finished = finish(&output(), listing.map_err(Error::from));

    assert!(
        finished.out.ends_with(
            "shelf has no aliases file, because neither XDG_CONFIG_HOME nor HOME is an absolute path.\n"
        ),
        "{}",
        finished.out
    );
}

#[test]
fn a_file_that_cannot_be_read_fails_with_its_path_and_the_reason() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"));

    let finished = finish(&output(), list(&output(), Some(directory), &args(&[])));

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
fn a_tool_that_has_not_kept_its_command_tree_cannot_list_its_aliases() {
    let finished = finish(
        &Output::default(),
        list(&Output::default(), None, &args(&[])),
    );

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(
        finished
            .err
            .ends_with("code:  crumbkit::aliases::unavailable\n"),
        "{}",
        finished.err
    );
}

/// The `crumbkit/aliases/v1` document as `finish` writes a listing of alias
/// rows with `--json`: the id, the rows and the number of aliases.
struct AliasesDocument;

impl JsonSchema for AliasesDocument {
    fn schema_name() -> Cow<'static, str> {
        "AliasesDocument".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit/aliases/v1".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The document that `aliases list` prints with --json: one row for each built-in alias and each entry of the aliases file.",
            "type": "object",
            "properties": {
                "schema": {
                    "description": "The id of this document.",
                    "const": "crumbkit/aliases/v1",
                },
                "data": {
                    "description": "The built-in aliases in command tree order, then the entries of the aliases file in name order, up to the limit.",
                    "type": "array",
                    "items": AliasRow::json_schema(generator),
                },
                "total": {
                    "description": "The number of built-in aliases and entries.",
                    "type": "integer",
                    "minimum": 0,
                },
            },
            "required": ["schema", "data", "total"],
        })
    }
}

#[test]
fn the_checked_in_schema_matches_the_aliases_document_schema() {
    let generated = serde_json::to_string_pretty(&schema_for!(AliasesDocument)).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/aliases-v1.json"
    ));

    assert_eq!(generated, checked_in);
}
