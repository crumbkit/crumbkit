//! The lines that describe a listing in text, which print on standard error
//! after the table: the footer of a listing cut short, the note of a listing
//! with no matching rows and the next step.

use std::io::{self, Write};

use clap::Command;

use crate::{Rows, Shape};

/// Returns whether `value` is a listing whose source holds rows that it
/// leaves out: fewer rows than its total, or more rows that the tool did not
/// count. Its JSON document then holds `"truncated": true`, as does a
/// document that the output budget cut short.
pub(super) fn leaves_out<T: Rows>(value: &T) -> bool {
    T::SHAPE == Shape::Listing && value.total().is_none_or(|total| value.rows().len() < total)
}

/// Writes the footer line that describes the table when `value` is a listing
/// that leaves out rows. A listing with a total gives the count, such as
/// `Showing 4 of 6 books. --all shows every one.`, and a listing without one
/// says that more match, such as `Showing 4 books, and more match. --all
/// shows every one.`, with `rows` when it has no description. A listing
/// whose `--limit` has a bound names it, as in `Showing 4 of 6 books.
/// --limit takes up to 100, and --all shows every one.` Otherwise it writes
/// nothing.
pub(super) fn write_footer<T: Rows>(out: &mut impl Write, value: &T) -> io::Result<()> {
    if !leaves_out(value) {
        return Ok(());
    }
    let total = value.total();
    write_shown(out, value.rows().len(), total, value.description())?;
    if total.is_none() {
        write!(out, ", and more match")?;
    }
    match value.limit_bound() {
        Some(most) => writeln!(
            out,
            ". --limit takes up to {most}, and --all shows every one."
        ),
        None => writeln!(out, ". --all shows every one."),
    }
}

/// Writes the start of the footer and of the cut line: `Showing {shown} of
/// {total} {description}`, with no description when there is none, or
/// `Showing {shown} {description}` without a total, with `rows` when there
/// is no description.
pub(super) fn write_shown(
    out: &mut impl Write,
    shown: usize,
    total: Option<usize>,
    description: Option<&str>,
) -> io::Result<()> {
    match (total, description) {
        (Some(total), Some(description)) => write!(out, "Showing {shown} of {total} {description}"),
        (Some(total), None) => write!(out, "Showing {shown} of {total}"),
        (None, description) => write!(out, "Showing {shown} {}", description.unwrap_or("rows")),
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
/// Otherwise it returns `None`: when the call names a
/// missing or hidden command or argument, the tool still prints its result
/// and crumbkit advertises nothing the tool hid. `finish` renders the line
/// once, before it writes the result, so each note it writes later uses the
/// same text.
pub(super) fn next_line<'a, T: Rows>(
    command: &mut Command,
    value: &'a T,
) -> Option<(String, &'a str)> {
    let (call, phrase) = value.next_step()?;
    let rendered = crate::call::render(call, command).ok()?;
    Some((rendered, phrase))
}
