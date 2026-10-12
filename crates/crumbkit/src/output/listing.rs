//! The lines that describe a listing in text, which print on standard error
//! after the table: the footer of a listing cut short, the note of a listing
//! with no matching rows and the next step.

use std::io::{self, Write};

use clap::Command;

use crate::Rows;

/// Writes the footer line that describes the table when `value` is a listing
/// that shows fewer rows than its total, such as `Showing 4 of 6 books. --all
/// shows every one.` A listing whose `--limit` has a bound names it, as in
/// `Showing 4 of 6 books. --limit takes up to 100, and --all shows every
/// one.` Otherwise it writes nothing.
pub(super) fn write_footer<T: Rows>(out: &mut impl Write, value: &T) -> io::Result<()> {
    let shown = value.rows().len();
    let Some(total) = value.total() else {
        return Ok(());
    };
    if shown >= total {
        return Ok(());
    }
    write!(out, "Showing {shown} of {total}")?;
    if let Some(description) = value.description() {
        write!(out, " {description}")?;
    }
    match value.limit_bound() {
        Some(most) => writeln!(
            out,
            ". --limit takes up to {most}, and --all shows every one."
        ),
        None => writeln!(out, ". --all shows every one."),
    }
}

/// Writes the note that describes the table when `value` is a listing with no
/// matching rows: `note: ` and `wording`, the document's own wording, or
/// `no {description} match`, such as `note: no books match`, or `note: no
/// rows match` for a listing without a description (ADR-0050). Otherwise it
/// writes nothing.
pub(super) fn write_empty_note<T: Rows>(
    out: &mut impl Write,
    value: &T,
    wording: Option<&str>,
) -> io::Result<()> {
    if value.total() != Some(0) {
        return Ok(());
    }
    match (wording, value.description()) {
        (Some(wording), _) => writeln!(out, "note: {wording}"),
        (None, Some(description)) => writeln!(out, "note: no {description} match"),
        (None, None) => writeln!(out, "note: no rows match"),
    }
}

/// Returns the rendered call and the phrase of the line `Next: {call}
/// {phrase}` when `value` is a listing with a next step whose call resolves in `command`.
/// Otherwise it returns `None`: without a command, or when the call names a
/// missing or hidden command or argument, the tool still prints its result
/// and crumbkit advertises nothing the tool hid. `finish` renders the line
/// once, before it writes the result, so each note it writes later uses the
/// same text.
pub(super) fn next_line<'a, T: Rows>(
    command: Option<&mut Command>,
    value: &'a T,
) -> Option<(String, &'a str)> {
    let (Some(command), Some((call, phrase))) = (command, value.next_step()) else {
        return None;
    };
    let rendered = crate::call::render(call, command).ok()?;
    Some((rendered, phrase))
}
