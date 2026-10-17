//! Tests for the exit code of a stream when the guide maps a failure's code
//! to one of the tool's exit codes (ADR-0037, ADR-0041).

use std::fmt;
use std::io;
use std::ops::ControlFlow;
use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};

use super::{SIGHTINGS, Tool, Writer, kingfisher, robin};
use crate::help::{Exit, Guide};
use crate::{Code, Failure, Output, Recovery, Sink, Spelling, stream_to};

const TIMED_OUT: Code = Code::new("birds", "watch", "timed_out");
const UNREADABLE: Code = Code::new("birds", "sightings", "unreadable");

/// A guide that maps `TIMED_OUT` to 124 and leaves `UNREADABLE` unmapped.
const GUIDE: Guide =
    Guide::new().exits(&[Exit::new(124, "The watch ran out of time.").failures(&[&TIMED_OUT])]);

/// A failure with the code it was given.
#[derive(Debug)]
struct Failed(&'static Code);

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the watch failed")
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

/// Parses `birds watch` and builds the output with `GUIDE`, as a tool that
/// parses with clap alone builds it.
fn guided() -> Output {
    let mut command = Tool::command();
    let matches = command
        .try_get_matches_from_mut(["birds", "watch"])
        .unwrap();
    let view = Tool::from_arg_matches(&matches).unwrap().view;
    Output::new(view, command, &GUIDE, ["birds", "watch"])
}

/// Streams one item, prints each of `failures` after it, and ends the
/// stream with `end`, and returns the exit code.
fn stream(failures: &[Failed], end: Result<(), Failed>) -> ExitCode {
    let mut output = guided();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();
    assert_eq!(streaming.item(&robin()), ControlFlow::Continue(()));
    for failure in failures {
        assert_eq!(streaming.failure(failure), ControlFlow::Continue(()));
    }
    streaming.end(end)
}

#[test]
fn a_stream_that_ends_with_a_mapped_failure_exits_with_its_declared_code() {
    assert_eq!(stream(&[], Err(Failed(&TIMED_OUT))), ExitCode::from(124));
}

#[test]
fn a_stream_that_ends_with_an_unmapped_failure_exits_with_code_1() {
    assert_eq!(stream(&[], Err(Failed(&UNREADABLE))), ExitCode::FAILURE);
}

#[test]
fn a_stream_that_printed_a_mapped_failure_and_ended_ok_exits_with_its_declared_code() {
    assert_eq!(stream(&[Failed(&TIMED_OUT)], Ok(())), ExitCode::from(124));
}

#[test]
fn a_stream_whose_first_failure_is_unmapped_exits_with_code_1_though_a_later_one_is_mapped() {
    let code = stream(&[Failed(&UNREADABLE)], Err(Failed(&TIMED_OUT)));
    assert_eq!(code, ExitCode::FAILURE);
}

#[test]
fn a_stream_whose_reader_closed_the_pipe_keeps_the_declared_code_of_its_failure() {
    let mut output = guided();
    let mut out = Writer::new(2, io::ErrorKind::BrokenPipe);
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, &mut output, Spelling::COLONS, SIGHTINGS).unwrap();

    assert_eq!(streaming.item(&robin()), ControlFlow::Continue(()));
    assert_eq!(streaming.item(&kingfisher()), ControlFlow::Break(()));
    let code = streaming.end(Err(Failed(&TIMED_OUT)));

    assert_eq!(code, ExitCode::from(124));
}
