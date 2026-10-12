//! The lines that follow a listing's table in text: the footer of a
//! listing cut short and the next step.

use std::io::{self, Write};

use clap::Command;

use crate::Rows;

/// Writes the footer line that follows the table when `value` is a listing
/// that shows fewer rows than its total, such as `Showing 4 of 6 books. --all
/// shows every one.` Otherwise it writes nothing.
pub(super) fn write_footer<T: Rows>(out: &mut impl Write, value: &T) -> io::Result<()> {
    let shown = value.rows().len();
    let Some(total) = value.total() else {
        return Ok(());
    };
    if shown >= total {
        return Ok(());
    }
    match value.description() {
        Some(description) => writeln!(
            out,
            "Showing {shown} of {total} {description}. --all shows every one."
        ),
        None => writeln!(out, "Showing {shown} of {total}. --all shows every one."),
    }
}

/// Returns the rendered call and the phrase of the line `Next: {call}
/// {phrase}` when `value` is a listing with a next step whose call resolves in `command`.
/// Otherwise it returns `None`: without a command, or when the call names a
/// missing or hidden command or argument, the tool still prints its result
/// and crumbkit advertises nothing the tool hid. `finish` renders the line
/// once, so it can measure the text and then write it.
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
