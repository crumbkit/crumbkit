//! Tests for the exit code of `finish_to` when the tool record maps a failure's
//! code to one of the tool's exit codes (ADR-0037).

use std::fmt;

use clap::{CommandFactory as _, Parser, Subcommand};

use super::{PLANETS, Planet, mercury};
use crate::answered::answered;
use crate::tool::{Exit, Failures};
use crate::{Batch, Code, Document, Failure, Output, Prose, Recovery, Sink, Tool, finish_to};

const TIMED_OUT: Code = Code::new("solar", "charts", "timed_out");
const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A tool record that maps `TIMED_OUT` to 124 and leaves `NOT_FOUND` unmapped.
const TOOL: Tool = Tool::new().failures(
    Failures::new()
        .exits(&[Exit::new(124, "The star chart ran out of time.").failures(&[&TIMED_OUT])]),
);

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
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: crate::ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// Parses `tool list` and builds the output with `TOOL`, as a tool that
/// parses with clap alone builds it.
fn output_of() -> Output {
    let view = Cli::try_parse_from(["tool", "list"]).unwrap().view;
    Output::new(
        view,
        crate::DryRun::default(),
        Cli::command(),
        &TOOL,
        ["tool", "list"],
    )
    .unwrap()
}

/// Finishes `result` into memory with `output`, and returns the exit code
/// as a number.
fn finish<T: crate::Rows>(output: Output, document: Document<T>, result: Result<T, Failed>) -> u8 {
    let mut out = Vec::new();
    let mut err = Vec::new();
    finish_to(&mut Sink::new(&mut out, &mut err), output, document, result)
}

const PLANETS_SHOW: Document<Batch<Planet, Failed>> =
    Document::new("solar", "planets-show", 1, "The test rows.");

#[test]
fn a_refusal_that_an_exit_lists_gives_its_stop_that_exit_as_a_number() {
    const REFUSALS: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(64, "The command line was refused.").failures(&[&crate::codes::CLI_USAGE]),
    ]));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let refused = crate::parse_to::<Cli>(
        &mut Sink::new(&mut out, &mut err),
        &REFUSALS,
        ["tool", "lst"],
    );
    let admitted = crate::parse_to::<Cli>(
        &mut Sink::new(&mut out, &mut err),
        &REFUSALS,
        ["tool", "list"],
    );

    assert_eq!(refused.err().map(|stop| stop.exit()), Some(64));
    assert!(admitted.is_ok());
}

#[test]
fn a_failure_whose_code_an_exit_lists_ends_finish_with_that_exit() {
    let code = finish(output_of(), PLANETS, Err(Failed(&TIMED_OUT)));
    assert_eq!(code, 124);
}

#[test]
fn a_failure_whose_code_no_exit_lists_ends_finish_with_code_1() {
    let code = finish(output_of(), PLANETS, Err(Failed(&NOT_FOUND)));
    assert_eq!(code, 1);
}

#[test]
fn an_output_set_from_a_bare_command_maps_no_failure_and_exits_with_code_1() {
    let view = Cli::try_parse_from(["tool", "list"]).unwrap().view;
    let output = Output::new(
        view,
        crate::DryRun::default(),
        Cli::command(),
        &Tool::new(),
        ["tool", "list"],
    )
    .unwrap();
    let code = finish(output, PLANETS, Err(Failed(&TIMED_OUT)));
    assert_eq!(code, 1);
}

#[test]
fn a_batch_whose_first_failure_is_mapped_exits_with_its_declared_code() {
    let batch = answered(
        vec![mercury()],
        vec![Failed(&TIMED_OUT), Failed(&NOT_FOUND)],
    );
    let code = finish(output_of(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, 124);
}

#[test]
fn a_batch_whose_first_failure_is_unmapped_exits_with_code_1_though_a_later_one_is_mapped() {
    let batch = answered(
        vec![mercury()],
        vec![Failed(&NOT_FOUND), Failed(&TIMED_OUT)],
    );
    let code = finish(output_of(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, 1);
}

#[test]
fn a_crumbkit_failure_in_a_batch_exits_with_the_code_that_the_tool_lists_for_it() {
    const LISTED: Tool = Tool::new().failures(
        Failures::new().exits(&[Exit::new(5, "The planet changed since it was read.")
            .failures(&[&crate::codes::VERSION_CONFLICT])]),
    );
    let view = Cli::try_parse_from(["tool", "list", "--json"])
        .unwrap()
        .view;
    let output = Output::new(
        view,
        crate::DryRun::default(),
        Cli::command(),
        &LISTED,
        ["tool", "list", "--json"],
    )
    .unwrap();
    let batch = answered(
        vec![mercury()],
        vec![Failed(&crate::codes::VERSION_CONFLICT)],
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = finish_to(
        &mut Sink::new(&mut out, &mut err),
        output,
        PLANETS_SHOW,
        Ok::<_, Failed>(batch),
    );

    assert_eq!(code, 5);
    let document: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(document["failures"][0]["code"], "tool::version::conflict");
}

#[test]
fn a_prose_result_whose_first_failure_is_mapped_exits_with_its_declared_code() {
    const SUMMARY: Document<Prose<String, Failed>> =
        Document::new("solar", "summary", 1, "The test summary.");
    let prose = Prose::with_failures(
        String::from("Mercury is the closest planet."),
        vec![Failed(&TIMED_OUT), Failed(&NOT_FOUND)],
    );
    let code = finish(output_of(), SUMMARY, Ok(prose));
    assert_eq!(code, 124);
}
