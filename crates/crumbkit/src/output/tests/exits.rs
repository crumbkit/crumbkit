//! Tests for the exit code of `finish_to` when the guide maps a failure's
//! code to one of the tool's exit codes (ADR-0037).

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};

use super::{PLANETS, Planet, mercury};
use crate::answered::answered;
use crate::help::{Exit, Guide, Leaf, Rename};
use crate::{
    Batch, Code, Document, Failure, Output, Parsed, Prose, Recovery, Sink, Spelling, finish_to,
};

const TIMED_OUT: Code = Code::new("solar", "charts", "timed_out");
const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A guide that maps `TIMED_OUT` to 124 and leaves `NOT_FOUND` unmapped.
const GUIDE: Guide = Guide::new()
    .exits(&[Exit::new(124, "The star chart ran out of time.").failures(&[&TIMED_OUT])]);

/// A failure with the code it was given.
#[derive(Debug)]
struct Failed(&'static Code);

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the star chart failed")
    }
}

impl std::error::Error for Failed {}

impl Failure for Failed {
    fn code(&self) -> &'static Code {
        self.0
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

/// A tool with a `list` command.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// Parses `tool list` and sets the tool's command with `GUIDE`, as a tool's
/// `main` does after `parse`.
fn guided() -> Output {
    let mut output = Tool::try_parse_from(["tool", "list"]).unwrap().output;
    output.set_command(Parsed::guided(Tool::command(), &GUIDE));
    output
}

/// Finishes `result` into memory with `output`, and returns the exit code.
fn finish<T: crate::Rows>(
    output: &mut Output,
    document: Document<T>,
    result: Result<T, Failed>,
) -> ExitCode {
    let mut out = Vec::new();
    let mut err = Vec::new();
    finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        Spelling::COLONS,
        document,
        result,
    )
}

const PLANETS_SHOW: Document<Batch<Planet, Failed>> =
    Document::new("solar", "planets-show", 1, "The test rows.");

#[test]
fn a_failure_whose_code_an_exit_lists_ends_finish_with_that_exit() {
    let code = finish(&mut guided(), PLANETS, Err(Failed(&TIMED_OUT)));
    assert_eq!(code, ExitCode::from(124));
}

#[test]
fn a_failure_whose_code_no_exit_lists_ends_finish_with_code_1() {
    let code = finish(&mut guided(), PLANETS, Err(Failed(&NOT_FOUND)));
    assert_eq!(code, ExitCode::FAILURE);
}

#[test]
fn an_output_set_from_a_bare_command_maps_no_failure_and_exits_with_code_1() {
    let mut output = Tool::try_parse_from(["tool", "list"]).unwrap().output;
    output.set_command(Tool::command());
    let code = finish(&mut output, PLANETS, Err(Failed(&TIMED_OUT)));
    assert_eq!(code, ExitCode::FAILURE);
}

#[test]
fn a_batch_whose_first_failure_is_mapped_exits_with_its_declared_code() {
    let batch = answered(
        vec![mercury()],
        vec![Failed(&TIMED_OUT), Failed(&NOT_FOUND)],
    );
    let code = finish(&mut guided(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, ExitCode::from(124));
}

#[test]
fn a_batch_whose_first_failure_is_unmapped_exits_with_code_1_though_a_later_one_is_mapped() {
    let batch = answered(
        vec![mercury()],
        vec![Failed(&NOT_FOUND), Failed(&TIMED_OUT)],
    );
    let code = finish(&mut guided(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, ExitCode::FAILURE);
}

#[test]
fn a_renamed_crumbkit_failure_in_a_batch_prints_the_tool_s_code_and_exits_with_its_listed_code() {
    const STALE: Code = Code::new("solar", "planets", "stale");
    const RENAMED: Guide = Guide::new()
        .codes(&[Rename::new(&crate::codes::VERSION_CONFLICT, &STALE)])
        .exits(&[Exit::new(5, "The planet changed since it was read.").failures(&[&STALE])]);
    let mut output = Tool::try_parse_from(["tool", "list", "--json"])
        .unwrap()
        .output;
    output.set_command(Parsed::guided(Tool::command(), &RENAMED));
    let batch = answered(
        vec![mercury()],
        vec![Failed(&crate::codes::VERSION_CONFLICT)],
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        PLANETS_SHOW,
        Ok::<_, Failed>(batch),
    );

    assert_eq!(code, ExitCode::from(5));
    let document: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(document["failures"][0]["code"], "solar::planets::stale");
}

#[test]
fn a_failure_of_a_leaf_that_renames_its_code_prints_the_leaf_s_code_and_exits_with_its_listed_code()
{
    const STALE: Code = Code::new("solar", "planets", "stale");
    const MOVED: Code = Code::new("solar", "planets", "moved");
    const RENAMED: Guide = Guide::new()
        .codes(&[Rename::new(&crate::codes::VERSION_CONFLICT, &STALE)])
        .exits(&[Exit::new(5, "A planet moved since it was read.").failures(&[&MOVED])])
        .leaves(&[
            Leaf::new(&["list"]).codes(&[Rename::new(&crate::codes::VERSION_CONFLICT, &MOVED)])
        ]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let (mut tool, parsed) = crate::parse_to::<Tool, _, _, _, _>(
        &mut sink,
        Tool::command(),
        &RENAMED,
        Spelling::COLONS,
        ["tool", "list", "--json"],
    )
    .unwrap();
    tool.output.set_command(parsed);
    let code = finish_to(
        &mut sink,
        &mut tool.output,
        Spelling::COLONS,
        PLANETS,
        Err(Failed(&crate::codes::VERSION_CONFLICT)),
    );

    assert_eq!(code, ExitCode::from(5));
    let document: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(document["code"], "solar::planets::moved");
}

#[test]
fn a_prose_result_whose_first_failure_is_mapped_exits_with_its_declared_code() {
    const SUMMARY: Document<Prose<String, Failed>> =
        Document::new("solar", "summary", 1, "The test summary.");
    let prose = Prose::with_failures(
        String::from("Mercury is the closest planet."),
        vec![Failed(&TIMED_OUT), Failed(&NOT_FOUND)],
    );
    let code = finish(&mut guided(), SUMMARY, Ok(prose));
    assert_eq!(code, ExitCode::from(124));
}
