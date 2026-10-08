//! Tests for the paging flags.

use clap::error::ErrorKind;
use clap::{CommandFactory as _, Parser, Subcommand};
use serde_json::json;

use super::Page;

/// A tool with two commands that flatten pages with different defaults.
#[derive(Parser)]
struct Tool {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// A command with the default page size.
    Books {
        #[command(flatten)]
        page: Page,
    },
    /// A command with a page size of its own.
    Authors {
        #[command(flatten)]
        page: Page<4>,
    },
}

/// Parses `tool books` followed by `flags`.
fn books(flags: &[&str]) -> Result<Page, clap::Error> {
    let args = ["tool", "books"].iter().chain(flags);
    match Tool::try_parse_from(args)?.command {
        Command::Books { page } => Ok(page),
        Command::Authors { .. } => unreachable!("the arguments name the books command"),
    }
}

/// Parses `tool authors` followed by `flags`.
fn authors(flags: &[&str]) -> Result<Page<4>, clap::Error> {
    let args = ["tool", "authors"].iter().chain(flags);
    match Tool::try_parse_from(args)?.command {
        Command::Authors { page } => Ok(page),
        Command::Books { .. } => unreachable!("the arguments name the authors command"),
    }
}

#[test]
fn a_command_that_flattens_page_passes_clap_debug_assertions() {
    Tool::command().debug_assert();
}

#[test]
fn the_limit_defaults_to_25_when_the_command_names_no_size() {
    assert_eq!(books(&[]).unwrap().limit(), Some(25));
}

#[test]
fn the_limit_defaults_to_the_size_the_command_names() {
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
}

#[test]
fn two_commands_with_different_sizes_each_keep_their_own_default() {
    // Parsing one command first must not change the other's default.
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
    assert_eq!(books(&[]).unwrap().limit(), Some(25));
    assert_eq!(authors(&[]).unwrap().limit(), Some(4));
}

#[test]
fn limit_sets_the_number_of_rows() {
    assert_eq!(books(&["--limit", "7"]).unwrap().limit(), Some(7));
}

#[test]
fn all_returns_no_limit() {
    assert_eq!(books(&["--all"]).unwrap().limit(), None);
}

#[test]
fn a_limit_of_0_is_refused() {
    let error = books(&["--limit", "0"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ValueValidation);
}

#[test]
fn all_conflicts_with_limit() {
    let error = books(&["--all", "--limit", "2"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
}

/// Returns the default of `--limit` that the subcommand `name` declares,
/// which clap's help shows as `[default: N]`.
fn declared_default(name: &str) -> String {
    let tool = Tool::command();
    let command = tool.find_subcommand(name).unwrap();
    let limit = command
        .get_arguments()
        .find(|arg| arg.get_id() == "limit")
        .unwrap();
    assert_eq!(limit.get_value_names().unwrap(), ["N"]);
    let [default] = limit.get_default_values() else {
        panic!("--limit has one default");
    };
    default.to_string_lossy().into_owned()
}

#[test]
fn each_command_declares_its_own_default_limit_for_its_help() {
    assert_eq!(declared_default("books"), "25");
    assert_eq!(declared_default("authors"), "4");
}

#[test]
fn a_page_serializes_its_limit_and_all() {
    let page = authors(&["--all"]).unwrap();

    assert_eq!(
        serde_json::to_value(&page).unwrap(),
        json!({"limit": 4, "all": true})
    );
}

#[test]
fn the_schema_describes_limit_with_its_default_and_all() {
    let schema = schemars::schema_for!(Page<4>).to_value();

    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["limit", "all"]));
    assert_eq!(schema["properties"]["limit"]["type"], "integer");
    assert_eq!(schema["properties"]["limit"]["minimum"], 1);
    assert_eq!(schema["properties"]["limit"]["default"], 4);
    assert_eq!(schema["properties"]["all"]["type"], "boolean");
    assert_eq!(schema["properties"]["all"]["default"], false);
}
