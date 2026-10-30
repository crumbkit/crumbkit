//! Tests for the view flags of a run whose handler fails: `parse` refuses
//! what the leaf's document cannot use and unknown `--fields` names, and
//! `finish` then prints the handler's failure with its own code whatever
//! document the tool finishes it in.

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::{Finished, PLANETS};
use crate::help::{Guide, Leaf};
use crate::{Code, Document, Failure, Prose, Recovery, Row, Rows, Sink, ViewFlags};
use crate::{finish_to, parse_to};

const NO_CONFIG: Code = Code::new("solar", "config", "not_found");

/// The document that the tool's early failures finish with, before the
/// command knows which result it prints.
const EARLY: Document<Prose<String>> = Document::new("solar", "early", 1, "The early answer.");

/// A setting, a row whose only field is `key`.
#[derive(Serialize, JsonSchema)]
struct Setting {
    key: &'static str,
}

impl Row for Setting {
    const COLUMNS: &'static [&'static str] = &["key"];
}

/// A document of rows that the tool's early failures may finish with, whose
/// row type lacks the fields of the leaf's declared document.
const SETTINGS: Document<Vec<Setting>> = Document::new("solar", "settings", 1, "The settings.");

/// A failure that the handler returns when it finds no config file.
#[derive(Debug)]
struct NoConfig;

impl fmt::Display for NoConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("config file not found")
    }
}

impl std::error::Error for NoConfig {}

impl Failure for NoConfig {
    fn code(&self) -> &'static Code {
        &NO_CONFIG
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("run the command inside the project".into())
    }
}

/// A tool with a `list` command and a `show` command.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
    Show,
}

/// A guide whose `list` leaf declares `PLANETS`, whose rows have a long
/// column, and whose `show` leaf declares no document.
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["list"]).document(PLANETS)]);

/// Parses `args` with `GUIDE`, and finishes the handler's `NoConfig` as
/// the tool's early failure, in the `EARLY` document.
fn fail_early(args: &[&str]) -> Finished {
    fail_in(args, EARLY)
}

/// Parses `args` with `GUIDE`, and finishes the handler's `NoConfig` as
/// the tool's early failure, in `document`.
fn fail_in<T: Rows>(args: &[&str], document: Document<T>) -> Finished {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let parsed = parse_to::<Tool, _, _, _, _>(&mut sink, Tool::command(), &GUIDE, args);
    let code = match parsed {
        Ok((_, output)) => {
            let result: Result<T, NoConfig> = Err(NoConfig);
            finish_to(&mut sink, output, document, result)
        }
        Err(stop) => stop.exit(),
    };
    Finished {
        code: ExitCode::from(code),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// The text of the handler's failure.
const NO_CONFIG_TEXT: &str = "\
error: config file not found
  help:  run the command inside the project
  code:  solar::config::not_found
";

#[test]
fn a_failed_run_prints_its_own_failure_when_long_is_valid_for_the_leaf_s_document() {
    let finished = fail_early(&["tool", "list", "--long"]);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, NO_CONFIG_TEXT);
}

#[test]
fn a_failed_run_prints_its_own_failure_when_fields_is_valid_for_the_leaf_s_document() {
    let finished = fail_early(&["tool", "list", "--fields", "name,ringed"]);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, NO_CONFIG_TEXT);
}

#[test]
fn a_failed_run_with_json_and_long_prints_the_failure_document() {
    let finished = fail_early(&["tool", "list", "--json", "--long"]);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.err, "");
    let document: serde_json::Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["schema"], "tool/failure/v1");
    assert_eq!(document["code"], "solar::config::not_found");
    assert_eq!(document["message"], "config file not found");
}

#[test]
fn a_failed_run_on_a_leaf_without_a_document_prints_its_own_failure_for_long() {
    let finished = fail_early(&["tool", "show", "--long"]);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.err, NO_CONFIG_TEXT);
}

#[test]
fn a_view_flag_that_the_leaf_s_document_cannot_use_is_still_refused_before_the_handler_runs() {
    const PROSE_GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["list"]).document(EARLY)]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);

    let parsed = parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        &PROSE_GUIDE,
        ["tool", "list", "--long"],
    );

    assert_eq!(parsed.err().map(|stop| stop.exit()), Some(2));
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "\
error: --long adds the long columns of a table, and this command prints prose
  help:  use --json to read every field of the result
  code:  tool::cli::usage
"
    );
}

#[test]
fn an_unknown_fields_name_on_a_leaf_with_a_document_is_refused_before_the_handler_runs() {
    let finished = fail_early(&["tool", "list", "--fields", "radius"]);

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert!(
        finished.err.starts_with(
            "error: --fields names an unknown field 'radius'\n  help:  use the valid fields: "
        ),
        "{}",
        finished.err
    );
    assert!(finished.err.contains("name"), "{}", finished.err);
    assert!(
        finished.err.ends_with("  code:  tool::cli::usage\n"),
        "{}",
        finished.err
    );
    assert!(!finished.err.contains("config file not found"));
}

#[test]
fn an_unknown_fields_name_on_a_leaf_without_a_document_prints_the_failure() {
    let finished = fail_early(&["tool", "show", "--fields", "radius"]);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.err, NO_CONFIG_TEXT);
}

#[test]
fn a_failed_run_finished_in_a_document_of_other_rows_prints_its_own_failure_for_fields() {
    let finished = fail_in(&["tool", "list", "--fields", "name"], SETTINGS);

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.out, "");
    assert_eq!(finished.err, NO_CONFIG_TEXT);
}
