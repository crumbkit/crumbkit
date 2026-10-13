//! Tests for the exit code of `finish_to` when the guide maps a failure's
//! code to one of the tool's exit codes (ADR-0037).

use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};

use super::{PLANETS, Planet, mercury};
use crate::help::{Exit, Guide};
use crate::{Batch, Code, Document, Failure, Output, Parsed, Recovery, Sink, Spelling, finish_to};

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
    let batch = Batch::new(
        vec![mercury()],
        vec![Failed(&TIMED_OUT), Failed(&NOT_FOUND)],
    );
    let code = finish(&mut guided(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, ExitCode::from(124));
}

#[test]
fn a_batch_whose_first_failure_is_unmapped_exits_with_code_1_though_a_later_one_is_mapped() {
    let batch = Batch::new(
        vec![mercury()],
        vec![Failed(&NOT_FOUND), Failed(&TIMED_OUT)],
    );
    let code = finish(&mut guided(), PLANETS_SHOW, Ok(batch));
    assert_eq!(code, ExitCode::FAILURE);
}
