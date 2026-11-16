//! Tests for `aliases list`, which lists the built-in aliases and the
//! entries of every source.

use std::path::Path;
use std::process::ExitCode;

use crumbkit::{Listing, Run, Sink, finish_to};
use serde_json::{Value, json};

use super::{ALIASES_LIST, AliasRow, AliasesList};
use crate::testing::tree;
use crate::{Aliases, Entry, Kind, Source};

/// The path that the personal rows name as their source.
const FILE: &str = "/home/ada/.config/shelf/aliases.toml";

/// Returns the handle of a run of the test tool with every view flag off.
fn output() -> Run {
    Run::new(
        crumbkit::ViewFlags::new(),
        crumbkit::DryRun::default(),
        tree(),
        &crumbkit::Tool::new(),
        ["shelf"],
    )
    .unwrap()
}

/// Parses `words` after `shelf aliases list` into the command's arguments.
fn args(words: &[&str]) -> AliasesList {
    let mut command = tree();
    let matches = command
        .try_get_matches_from_mut(["shelf", "aliases", "list"].iter().chain(words))
        .unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    let (_, matches) = matches.subcommand().unwrap();
    <AliasesList as clap::FromArgMatches>::from_arg_matches(matches).unwrap()
}

/// Returns the aliases of a personal file at [`FILE`] that holds `text`.
fn personal(text: &str) -> Aliases {
    Aliases::new().source(Source::text_at(FILE, Kind::Personal, Some(text)))
}

/// Returns the rows of `aliases`, as JSON.
fn rows_of(aliases: &Aliases) -> Value {
    serde_json::to_value(aliases.listing(&tree(), &args(&[]))).unwrap()
}

/// Returns the rows for a personal file at [`FILE`] that holds `text`, as
/// JSON.
fn listed(text: &str) -> Value {
    rows_of(&personal(text))
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

/// Finishes `listing` into memory as a tool's handler does.
fn finish(output: Run, listing: Listing<AliasRow>) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        ALIASES_LIST,
        Ok::<_, crumbkit::Error>(listing),
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
    let aliases = Aliases::new().source(Source::file(Some(missing.clone())));

    let listing = aliases.listing(&tree(), &args(&["--limit", "1"]));
    let finished = finish(output(), listing);

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
    let listing = personal("").listing(&tree(), &args(&[]));

    let finished = finish(output(), listing);

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
    let listing = personal(text).listing(&tree(), &args(&["--limit", "1"]));

    let finished = finish(output(), listing);

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
    let aliases = Aliases::new().source(Source::file(None));

    let finished = finish(output(), aliases.listing(&tree(), &args(&[])));

    assert_eq!(
        finished.err,
        "shelf has no aliases file, because the home directory cannot be found.\n"
    );
}

#[test]
fn a_file_that_cannot_be_read_shows_one_problem_row_and_the_command_exits_with_1() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let aliases = Aliases::new().source(Source::file(Some(directory.to_path_buf())));

    let finished = finish(output(), aliases.listing(&tree(), &args(&[])));

    assert_eq!(finished.code, ExitCode::FAILURE);
    let line = finished.out.lines().nth(1).unwrap();
    assert!(
        line.starts_with(&directory.display().to_string()),
        "{}",
        finished.out
    );
    assert!(line.contains("cannot be read: "), "{}", finished.out);
    assert_eq!(finished.out.lines().count(), 4, "{}", finished.out);
}

#[test]
fn a_file_that_cannot_be_parsed_shows_a_problem_row_with_the_line_and_the_parser_s_message() {
    let rows = listed("mine = [\"books\"]\nours = [");

    let problem = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["from"] == FILE)
        .unwrap();
    assert_eq!(problem["name"], FILE);
    assert!(
        problem["problem"]
            .as_str()
            .unwrap()
            .starts_with("cannot be read: line 2: "),
        "{problem}"
    );
    assert_eq!(rows.as_array().unwrap().len(), 3);
}

#[test]
fn the_rows_of_every_source_carry_the_origin_of_their_source() {
    fn entries() -> Result<Vec<Entry>, String> {
        Ok(vec![Entry::new("repo run", ["books", "show"])])
    }
    let aliases = Aliases::new()
        .source(Source::text_at(
            FILE,
            Kind::Personal,
            Some(r#"mine = ["books"]"#),
        ))
        .source(Source::table(
            "shelf defaults",
            &[("wt ls", &["books", "list"])],
        ))
        .source(Source::lazy("repository", Kind::Shipped, entries));

    let rows = rows_of(&aliases);

    assert_eq!(row(&rows, "mine")["from"], FILE);
    assert_eq!(row(&rows, "wt ls")["from"], "shelf defaults");
    assert_eq!(row(&rows, "repo run")["from"], "repository");
    assert_eq!(row(&rows, "wt ls")["command"], "books list");
}

#[test]
fn a_source_that_cannot_be_read_does_not_hide_the_rows_of_the_others() {
    fn fails() -> Result<Vec<Entry>, String> {
        Err("no network".to_owned())
    }
    let aliases = Aliases::new()
        .source(Source::lazy("repository", Kind::Shipped, fails))
        .source(Source::table("shelf", &[("wt", &["books", "list"])]));

    let rows = rows_of(&aliases);

    assert_eq!(
        row(&rows, "repository")["problem"],
        "cannot be read: no network"
    );
    assert_eq!(row(&rows, "wt")["problem"], "");
}

#[test]
fn a_key_that_names_a_real_command_path_says_the_real_one_wins() {
    let aliases = Aliases::new().source(Source::table(
        "shelf",
        &[
            ("books list", &["books", "show"]),
            ("books recent", &["books", "show"]),
        ],
    ));

    let rows = rows_of(&aliases);

    assert_eq!(
        row(&rows, "books list")["problem"],
        "the tool has a command or built-in alias of this name, which wins"
    );
    assert_eq!(row(&rows, "books recent")["problem"], "");
}

#[test]
fn a_file_key_that_starts_with_a_command_says_a_file_cannot_start_with_one() {
    let rows = listed("\"books recent\" = [\"books\", \"show\"]");

    assert_eq!(
        row(&rows, "books recent")["problem"],
        "the first word is a command of the tool, and entries of a file or a lazy source cannot start with one"
    );
}

#[test]
fn a_key_that_starts_with_help_says_the_tool_answers_help_itself() {
    let rows = listed(r#""help me" = ["books", "list"]"#);

    assert_eq!(
        row(&rows, "help me")["problem"],
        "the tool answers the word help with its own help, which wins"
    );
}

#[test]
fn a_key_without_words_says_it_needs_one() {
    let rows = listed(r#""  " = ["books", "list"]"#);

    assert_eq!(
        rows.as_array()
            .unwrap()
            .iter()
            .find(|row| row["from"] == FILE)
            .unwrap()["problem"],
        "the key must be one or more words"
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

#[test]
fn aliases_list_s_document_is_marked_as_crumbkit_s_own() {
    assert!(ALIASES_LIST.is_built_in());
}
