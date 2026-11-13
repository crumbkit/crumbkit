//! The exit code that a tool outside crumbkit reads from a `Stop` as a
//! number, which it records or passes on because an `ExitCode` cannot be
//! read back.

use std::io::{self, Write};

use clap::{Parser, Subcommand};
use crumbkit::tool::{Exit, Failures};
use crumbkit::{Sink, Tool, ViewFlags};

/// A tool record that lists exit 64 for a refused command line.
const TOOL: Tool = Tool::new().failures(Failures::new().exits(&[
    Exit::new(64, "The command line was refused.").failures(&[crumbkit::codes::CLI_USAGE]),
]));

/// A tool with a version and a `list` command.
#[derive(Parser)]
#[command(name = "tool", version = "1.2.3")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// A standard output that refuses every write, as a full disk does. A
/// closed pipe would end the run with 0, because the reader has gone.
struct Full;

impl Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::StorageFull))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::StorageFull))
    }
}

/// Parses `args` with `TOOL`, writing to `out` and to a standard error in
/// memory, and returns the number of the `Stop` that parsing ended with, or
/// `None` when the arguments parsed.
fn number_to(out: impl Write, args: &[&str]) -> Option<u8> {
    let mut err = Vec::new();
    let parsed = crumbkit::parse_to::<Cli>(&mut Sink::new(out, &mut err), &TOOL, args);
    parsed.err().map(|stop| stop.exit())
}

/// Parses `args` with `TOOL` into memory and returns the number of the
/// `Stop` that parsing ended with, or `None` when the arguments parsed.
fn number(args: &[&str]) -> Option<u8> {
    number_to(Vec::new(), args)
}

#[test]
fn a_stops_number_is_the_exit_code_the_tool_lists_for_a_refused_command_line() {
    assert_eq!(number(&["tool", "lst"]), Some(64));
}

#[test]
fn a_stops_number_is_0_after_help() {
    assert_eq!(number(&["tool", "--help"]), Some(0));
}

#[test]
fn a_stops_number_is_0_after_the_version() {
    assert_eq!(number(&["tool", "--version"]), Some(0));
}

#[test]
fn a_stops_number_is_1_when_the_help_page_cannot_be_written() {
    assert_eq!(number_to(Full, &["tool", "--help"]), Some(1));
}
