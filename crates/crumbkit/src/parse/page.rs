//! The printing of a help page or the version, and the exit code that it
//! ends the run with.

use std::fmt::Display;
use std::io::{self, Write};
use std::process::ExitCode;

use crate::Sink;

/// Prints the help page or the version in `error` and returns the exit code:
/// 0 when it is written or the reader closed the pipe early, and 1 when the
/// write fails for another reason. With `colour`, clap's `Error::print`
/// writes it to standard output with clap's colours, and otherwise it goes
/// to the sink's output writer as plain text.
pub(super) fn print_page<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    error: &clap::Error,
    colour: bool,
) -> ExitCode {
    if colour {
        let written = error.print();
        exit_code(sink, written)
    } else {
        print_text(sink, &error.render())
    }
}

/// Prints `text` to the sink's output writer as plain text and returns the
/// exit code, as [`print_page`] does.
pub(super) fn print_text<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    text: &dyn Display,
) -> ExitCode {
    let written = write!(sink.out, "{text}").and_then(|()| sink.out.flush());
    exit_code(sink, written)
}

/// Returns the exit code after a page was `written`, and reports a failed
/// write on the sink's error writer.
fn exit_code<O: Write, E: Write, R>(sink: &mut Sink<O, E, R>, written: io::Result<()>) -> ExitCode {
    match written {
        Ok(()) => ExitCode::SUCCESS,
        // A reader that stops early, as `shelf -h | head -1` does, closes the
        // pipe after it has read every line it wanted.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            ExitCode::FAILURE
        }
    }
}
