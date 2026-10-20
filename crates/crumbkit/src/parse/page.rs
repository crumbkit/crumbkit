//! The printing of a help page or the version, and the exit code that it
//! ends the run with. Help and the version are the run's result, so the sink
//! delivers them to its output writer (ADR-0128).

use std::fmt::Write as _;
use std::io;

use clap::builder::StyledStr;

use crate::sink::role::{Deliver, Role};

/// Prints the help page or the version in `error` as the result and returns
/// the exit code as a number: 0 when it is written or the reader closed the
/// pipe early, and 1 when the write fails for another reason. `Stop::code`
/// turns the number into an `ExitCode`.
///
/// A sink that asks for no styles for the result receives the page as plain
/// text. For a styled sink over the process's standard streams, which
/// `standard` says, clap's `Error::print` writes the page to standard
/// output, so clap's `NO_COLOR` handling and the command's `ColorChoice`
/// apply (ADR-0021). Any other styled sink receives the page with clap's
/// ANSI escape codes, the bytes that `Error::print` writes at a terminal.
pub(super) fn print_page(sink: &mut dyn Deliver, error: &clap::Error, standard: bool) -> u8 {
    let styled = sink.styles(Role::Result);
    if styled && standard {
        let written = error.print();
        exit_code(sink, written)
    } else {
        print_text(sink, &rendered(&error.render(), styled))
    }
}

/// Returns `page` as text: with clap's ANSI escape codes when `styled` is
/// set and crumbkit's `color` feature is on, and as plain text otherwise.
/// clap offers a page's escape codes only with its `color` feature, which
/// crumbkit's `color` feature turns on.
pub(crate) fn rendered(page: &StyledStr, styled: bool) -> String {
    let mut text = String::with_capacity(PAGE);
    #[cfg(feature = "color")]
    let written = if styled {
        write!(text, "{}", page.ansi())
    } else {
        write!(text, "{page}")
    };
    #[cfg(not(feature = "color"))]
    let written = {
        let _ = styled;
        write!(text, "{page}")
    };
    // Writing to a `String` never fails.
    debug_assert!(written.is_ok(), "a String takes every write");
    text
}

/// The capacity that a page is rendered in, which holds a help page of
/// ordinary length without growing.
const PAGE: usize = 4096;

/// Delivers `text` as the result and returns the exit code as a number, as
/// [`print_page`] does.
pub(super) fn print_text(sink: &mut dyn Deliver, text: &str) -> u8 {
    let written = sink.deliver(Role::Result, text.as_bytes());
    exit_code(sink, written)
}

/// Returns the exit code as a number after a page was `written`, and
/// delivers the report of a failed write.
fn exit_code(sink: &mut dyn Deliver, written: io::Result<()>) -> u8 {
    match written {
        Ok(()) => 0,
        // A reader that stops early, as `shelf -h | head -1` does, closes the
        // pipe after it has read every line it wanted.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => 0,
        Err(error) => {
            sink.report_failed_write(&error);
            1
        }
    }
}
