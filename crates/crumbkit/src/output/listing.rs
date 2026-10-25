//! The lines that describe a listing in text: the summary lines, which print
//! on standard output above the table, and the lines that print on standard
//! error after the table, which are the footer of a listing cut short, the
//! note of a listing with no matching rows and the next step.

use std::io::{self, Write};

use clap::Command;

use super::table::escape_controls;
use crate::{Rows, Shape};

/// Returns whether `value` is a listing whose source holds rows that it
/// leaves out: fewer rows than its total, more rows that the tool did not
/// count, or rows beyond those the tool read from a capped source. Its JSON document then holds `"truncated": true`, as does a
/// document that the output budget cut short.
pub(super) fn leaves_out<T: Rows>(value: &T) -> bool {
    T::SHAPE == Shape::Listing && value.more()
}

/// Returns whether `value` is a listing with no matching rows: it holds no
/// rows, and its source holds none that it leaves out.
pub(super) fn matches_none<T: Rows>(value: &T) -> bool {
    T::SHAPE == Shape::Listing && value.rows().is_empty() && !value.more()
}

/// Writes the footer line that describes the table when `value` is a listing
/// that leaves out rows. A listing with a total gives the count, such as
/// `Showing 4 of 6 books. --all shows every one.`, and a listing without one
/// says that more match, such as `Showing 4 books, and more match. --all
/// shows every one.`, with `rows` when it has no description. Such a
/// listing names one shown row with its singular, as in `Showing 1 book,
/// and more match.`, or as `row` when it has no description. A listing
/// whose `--limit` has a bound names it, as in `Showing 4 of 6 books.
/// --limit takes up to 100, and --all shows every one.` A listing from a
/// capped source writes [`write_capped_footer`]'s line. Otherwise it writes
/// nothing.
pub(super) fn write_footer<T: Rows>(out: &mut impl Write, value: &T) -> io::Result<()> {
    if !leaves_out(value) {
        return Ok(());
    }
    if let Some(read) = value.at_least() {
        return write_capped_footer(out, value, read);
    }
    let total = value.total();
    write_shown(
        out,
        value.rows().len(),
        total,
        value.description(),
        value.singular(),
    )?;
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

/// Writes the footer of a listing from a capped source, whose tool read
/// `read` rows and whose source holds more (ADR-0130). While the page
/// shows fewer than `read` rows, it gives `read` as a lower bound and says
/// how many rows `--all` shows, as in `Showing 20 of at least 100 records.
/// --all shows 100 of them.`, and names the bound of `--limit` when it has
/// one. When the page shows every row that the tool read, `--all` shows
/// nothing more, so the footer reads `Showing 100 records, and more
/// match.`, with the singular or `rows` as a listing without a total uses.
fn write_capped_footer<T: Rows>(out: &mut impl Write, value: &T, read: usize) -> io::Result<()> {
    let shown = value.rows().len();
    if shown >= read {
        write_shown(out, shown, None, value.description(), value.singular())?;
        return writeln!(out, ", and more match.");
    }
    match value.description() {
        Some(description) => write!(out, "Showing {shown} of at least {read} {description}")?,
        None => write!(out, "Showing {shown} of at least {read}")?,
    }
    match value.limit_bound() {
        Some(most) => writeln!(
            out,
            ". --limit takes up to {most}, and --all shows {read} of them."
        ),
        None => writeln!(out, ". --all shows {read} of them."),
    }
}

/// Writes the start of the footer and of the cut line: `Showing {shown} of
/// {total} {description}`, with no description when there is none, or
/// `Showing {shown} {description}` without a total, with `rows` when there
/// is no description. Without a total, one shown row takes `singular`, or
/// `row` when there is no description, so the line reads `Showing 1 book`.
pub(super) fn write_shown(
    out: &mut impl Write,
    shown: usize,
    total: Option<usize>,
    description: Option<&str>,
    singular: Option<&str>,
) -> io::Result<()> {
    let Some(total) = total else {
        let noun = match (shown, singular, description) {
            (1, Some(one), _) => one,
            (1, None, None) => "row",
            (_, _, Some(many)) => many,
            (_, _, None) => "rows",
        };
        return write!(out, "Showing {shown} {noun}");
    };
    match description {
        Some(description) => write!(out, "Showing {shown} of {total} {description}"),
        None => write!(out, "Showing {shown} of {total}"),
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
    if !matches_none(value) {
        return Ok(());
    }
    match (wording, value.description()) {
        (Some(wording), _) => writeln!(out, "note: {wording}"),
        (None, Some(description)) => writeln!(out, "note: no {description} match"),
        (None, None) => writeln!(out, "note: no rows match"),
    }
}

/// Returns the lines that name the next steps of `value`, each with its line
/// feed: the line `Next: {call} {phrase}` of a listing's next step, then the
/// lines of each of its further steps ([`crate::Step`]). A next step whose
/// call does not resolve in `command`, and a step with such a call, is left
/// out: when a call names a missing or hidden command or argument, the tool
/// still prints its result and crumbkit advertises nothing the tool hid.
/// `finish` renders the lines once, before it writes the result, so each
/// note it writes later uses the same text.
pub(super) fn next_lines<T: Rows>(command: &mut Command, value: &T) -> String {
    let mut lines = String::new();
    if let Some((call, phrase)) = value.next_step()
        && let Ok(rendered) = crate::call::render(call, command)
    {
        lines = format!("Next: {rendered} {phrase}\n");
    }
    for step in value.steps() {
        if let Some(step_lines) = step.lines(command) {
            lines.push_str(&step_lines);
        }
    }
    lines
}

/// Returns the lines of `summary`, the text of a summary's `Display`, as the
/// text prints them above the table (ADR-0158). Each line feed ends a line,
/// and a final line feed ends the last line without adding an empty one.
/// Each other control character and each Unicode format character is
/// escaped as a table cell escapes it, and each line ends with a line feed.
pub(super) fn summary_lines(summary: &str) -> String {
    let body = summary.strip_suffix('\n').unwrap_or(summary);
    let mut lines = String::with_capacity(body.len() + 1);
    for line in body.split('\n') {
        lines.push_str(&escape_controls(line));
        lines.push('\n');
    }
    lines
}
