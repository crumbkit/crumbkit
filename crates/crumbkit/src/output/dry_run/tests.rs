//! Tests for `--dry-run` and how `finish_to` marks a dry run.

use std::process::ExitCode;

use clap::{Args as _, Command, CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::DryRun;
use crate::{Document, Error, Output, Prose, Row, Sink, Spelling, finish_to};

/// A removed planet.
#[derive(Serialize, JsonSchema)]
struct Removed {
    name: &'static str,
}

impl Row for Removed {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const REMOVED: Document<Vec<Removed>> =
    Document::new("solar", "planets-remove", 1, "The removed planets.");

/// A tool whose `remove` command takes `--dry-run`.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Commands,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Commands {
    Remove {
        #[command(flatten)]
        dry_run: DryRun,
    },
}

/// What `finish_to` returned and wrote.
struct Finished {
    dry_run: bool,
    code: ExitCode,
    out: String,
}

/// Parses `tool remove` followed by `flags`, records the dry run, and
/// finishes `result` into memory with `document`.
fn finish_with<T: crate::Rows>(
    flags: &[&str],
    document: Document<T>,
    result: Result<T, Error>,
) -> Finished {
    let args = ["tool", "remove"].iter().chain(flags);
    let tool = Tool::try_parse_from(args).unwrap();
    let mut output = tool.output;
    let Commands::Remove { dry_run } = tool.command;
    let dry_run = output.dry_run(dry_run);
    output.set_command(Tool::command());
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        document,
        result,
    );
    Finished {
        dry_run,
        code,
        out: String::from_utf8(out).unwrap(),
    }
}

fn pluto() -> Vec<Removed> {
    vec![Removed { name: "Pluto" }]
}

#[test]
fn dry_run_returns_whether_the_caller_passed_dry_run() {
    assert!(finish_with(&["--dry-run"], REMOVED, Ok(pluto())).dry_run);
    assert!(!finish_with(&[], REMOVED, Ok(pluto())).dry_run);
}

#[test]
fn a_dry_run_s_document_holds_dry_run_true_right_after_its_schema() {
    let finished = finish_with(&["--dry-run", "--json"], REMOVED, Ok(pluto()));

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(
        finished.out,
        "{\"schema\":\"solar/planets-remove/v1\",\"dry_run\":true,\"data\":[{\"name\":\"Pluto\"}]}\n"
    );
}

#[test]
fn a_run_without_dry_run_prints_no_dry_run_field() {
    let finished = finish_with(&["--json"], REMOVED, Ok(pluto()));

    assert_eq!(
        finished.out,
        "{\"schema\":\"solar/planets-remove/v1\",\"data\":[{\"name\":\"Pluto\"}]}\n"
    );
}

#[test]
fn a_dry_run_in_text_ends_with_a_line_that_says_nothing_changed() {
    let finished = finish_with(&["--dry-run"], REMOVED, Ok(pluto()));

    assert_eq!(
        finished.out,
        "NAME\nPluto\nThis was a dry run, so nothing changed.\n"
    );
}

#[test]
fn a_dry_run_s_prose_ends_with_the_same_line() {
    const SAID: Document<Prose<&str>> = Document::new("solar", "said", 1, "A sentence.");
    let finished = finish_with(&["--dry-run"], SAID, Ok(Prose::new("Pluto would go.")));

    assert_eq!(
        finished.out,
        "Pluto would go.\nThis was a dry run, so nothing changed.\n"
    );
}

#[test]
fn dry_run_is_the_flag_dry_run_with_crumbkit_s_id_and_serializes_as_one_field() {
    let command = DryRun::augment_args(Command::new("remove"));
    let arg = command.get_arguments().next().unwrap();
    assert_eq!(arg.get_id(), "crumbkit_dry_run");
    assert_eq!(arg.get_long(), Some("dry-run"));

    let tool = Tool::try_parse_from(["tool", "remove", "--dry-run"]).unwrap();
    let Commands::Remove { dry_run } = tool.command;
    assert_eq!(
        serde_json::to_string(&dry_run).unwrap(),
        "{\"dry_run\":true}"
    );
    assert_eq!(
        schemars::schema_for!(DryRun).to_value()["properties"]["dry_run"]["type"],
        "boolean"
    );
}

#[test]
fn a_dry_run_of_a_document_without_the_envelope_prints_it_unchanged_and_says_so_on_standard_error()
{
    let args = ["tool", "remove", "--dry-run", "--json"];
    let tool = Tool::try_parse_from(args).unwrap();
    let mut output = tool.output;
    let Commands::Remove { dry_run } = tool.command;
    output.dry_run(dry_run);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<_, Error> = Ok(pluto());
    finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        REMOVED.without_envelope(),
        result,
    );

    assert_eq!(String::from_utf8(out).unwrap(), "[{\"name\":\"Pluto\"}]\n");
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "This was a dry run, so nothing changed.\n"
    );
}
