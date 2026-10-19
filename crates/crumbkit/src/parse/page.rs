//! The printing of a help page or the version, and the exit code that it
//! ends the run with.

use std::fmt::Display;
use std::io::{self, Write};

use clap::builder::StyledStr;

use crate::Sink;

/// Prints the help page or the version in `error` and returns the exit code
/// as a number: 0 when it is written or the reader closed the pipe early,
/// and 1 when the write fails for another reason. `Stop::code` turns the
/// number into an `ExitCode`.
///
/// A sink that asks for no styles receives the page as plain text. For a
/// styled sink over the process's standard streams, which `standard` says,
/// clap's `Error::print` writes the page to standard output, so clap's
/// `NO_COLOR` handling and the command's `ColorChoice` apply. Any other
/// styled sink receives the page with clap's ANSI escape codes on its output
/// writer, the bytes that `Error::print` writes at a terminal.
pub(super) fn print_page<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    error: &clap::Error,
    standard: bool,
) -> u8 {
    if !sink.is_styled() {
        print_text(sink, &error.render())
    } else if standard {
        let written = error.print();
        exit_code(sink, written)
    } else {
        print_styled(sink, &error.render())
    }
}

/// Prints `page` to the sink's output writer with its ANSI escape codes and
/// returns the exit code as a number, as [`print_page`] does.
#[cfg(feature = "color")]
fn print_styled<O: Write, E: Write, R>(sink: &mut Sink<O, E, R>, page: &StyledStr) -> u8 {
    let written = write!(sink.out, "{}", page.ansi()).and_then(|()| sink.out.flush());
    exit_code(sink, written)
}

/// Prints `page` to the sink's output writer as plain text, because clap
/// offers a page's escape codes only with its `color` feature, which
/// crumbkit's `color` feature turns on.
#[cfg(not(feature = "color"))]
fn print_styled<O: Write, E: Write, R>(sink: &mut Sink<O, E, R>, page: &StyledStr) -> u8 {
    print_text(sink, page)
}

/// Prints `text` to the sink's output writer as plain text and returns the
/// exit code as a number, as [`print_page`] does.
pub(super) fn print_text<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    text: &dyn Display,
) -> u8 {
    let written = write!(sink.out, "{text}").and_then(|()| sink.out.flush());
    exit_code(sink, written)
}

/// Returns the exit code as a number after a page was `written`, and
/// reports a failed write on the sink's error writer.
fn exit_code<O: Write, E: Write, R>(sink: &mut Sink<O, E, R>, written: io::Result<()>) -> u8 {
    match written {
        Ok(()) => 0,
        // A reader that stops early, as `shelf -h | head -1` does, closes the
        // pipe after it has read every line it wanted.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            1
        }
    }
}
