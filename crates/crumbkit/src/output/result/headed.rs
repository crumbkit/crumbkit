//! How the text of a batch whose document is headed prints: each answer's
//! bytes under a `==> KEY <==` header, within the output budget (ADR-0062).

use std::borrow::Cow;
use std::io::{self, Write};

use super::Answer;
use super::rerun::{self, Rerun};
use crate::output::budget::{Over, OverBudget};
use crate::output::table::escape_controls;
use crate::output::{Output, layout, problems};
use crate::{Code, Rows, codes};

/// Writes the rows of `value` to `out` in the text of a headed document,
/// within the bytes that [`Output::budget`] has left, then flushes `out`.
/// `bytes_of` returns the bytes of the row at a position, or `None` for a
/// row that holds none. It returns what it printed and the number of bytes
/// it wrote.
///
/// Each row prints whole or not at all. A row prints its bytes unchanged,
/// or one line of compact JSON with every field when it holds no bytes.
/// When the batch has more than one key, a header line `==> KEY <==`
/// precedes each row, with the key's control and format characters escaped,
/// as a table cell escapes them, and a
/// line feed precedes each header after the first, as `head` prints many
/// files. A key's failure prints no header.
///
/// The first row prints when its header and content fit in the budget, or
/// when `first_always` is set because the document always prints its first
/// answer. Otherwise no row prints, the first row gets its own over-budget
/// failure under its key, and every later row and every key the batch
/// never fetched is left out. The later rows print in order while they
/// fit. From the first later row that does not fit, alone or beside the
/// rows before it, that row, every later row and every key the batch never
/// fetched are left out. One failure names the keys left out, and the
/// JSON form follows the same rule (ADR-0084). After `out` is flushed, the
/// note of the rows that report a problem, when every answer printed, and
/// the tool's note go to `err`, as they do for a table.
pub(super) fn write<T: Rows>(
    out: &mut impl Write,
    err: &mut impl Write,
    output: &Output,
    value: &T,
    bytes_of: fn(&T, usize) -> Option<&[u8]>,
    problems: usize,
    first_always: bool,
) -> io::Result<(Answer, usize)> {
    let budget = output.budget();
    let code = output.renames.printed(&codes::OUTPUT_OVER_BUDGET);
    let rows = value.rows();
    let headers = crate::output::several_keys(value) || rows.len() > 1;
    let mut printed: Vec<(String, Cow<'_, [u8]>)> = Vec::new();
    let mut spent = 0_usize;
    let mut stop = rows.len();
    let mut refused = None;
    for (index, row) in rows.iter().enumerate() {
        let position = value.row_indexes().get(index).copied().unwrap_or(index);
        let header = if headers {
            header(value.key(position), position)
        } else {
            String::new()
        };
        let content = if let Some(bytes) = bytes_of(value, index) {
            Cow::Borrowed(bytes)
        } else {
            let mut line = Vec::new();
            layout::serialize(&mut line, row)?;
            line.push(b'\n');
            Cow::Owned(line)
        };
        let separator = usize::from(!printed.is_empty());
        let cost = header
            .len()
            .saturating_add(content.len())
            .saturating_add(separator);
        if index == 0 && cost > budget && !first_always {
            refused = Some(position);
            stop = 1;
            break;
        }
        if index > 0 && spent.saturating_add(cost) > budget {
            stop = index;
            break;
        }
        spent = spent.saturating_add(cost);
        printed.push((header, content));
    }
    for (number, (header, content)) in printed.iter().enumerate() {
        if number > 0 {
            out.write_all(b"\n")?;
        }
        out.write_all(header.as_bytes())?;
        out.write_all(content)?;
    }
    out.flush()?;
    // The caller's words are parsed again only when an answer is left out.
    let rerun = if value.left_out(stop).is_empty() {
        None
    } else {
        Rerun::find(&output.command, &output.words, value)
    };
    let left_out = left_out(value, stop, code, budget, rerun.as_ref());
    // A failure for an answer left out decides the exit, so the note of the
    // rows that report a problem, which names the exit, prints only when
    // every answer printed. The tool's note always prints.
    let problems = if left_out.is_none() && refused.is_none() {
        problems
    } else {
        0
    };
    write_notes(err, value, problems);
    let answer = match refused {
        Some(position) => Answer::RefusedFirst {
            position,
            first: OverBudget {
                code,
                budget,
                over: Over::Whole,
            },
            left_out,
        },
        None => left_out.map_or(Answer::Whole, Answer::Partial),
    };
    Ok((answer, spent))
}

/// Returns the header line of the row at `position`, whose key is `key`:
/// `==> KEY <==` with the key's control and format characters escaped, or the
/// position in parentheses for a row pushed after the last key.
pub(in crate::output) fn header(key: Option<&str>, position: usize) -> String {
    match key {
        Some(key) => format!("==> {} <==\n", escape_controls(key)),
        None => format!("==> (index {position}) <==\n"),
    }
}

/// Returns the failure of the rows from the row numbered `stop` on and the
/// keys the batch never fetched, which names every key, or `None` when no
/// row or key is left out. Its recovery reruns the caller's words from
/// `rerun` with the keys left out, or names the batch's command alone.
fn left_out<T: Rows>(
    value: &T,
    stop: usize,
    code: &'static Code,
    budget: usize,
    rerun: Option<&Rerun>,
) -> Option<OverBudget> {
    let keys = value.left_out(stop);
    if keys.is_empty() {
        return None;
    }
    let named: Vec<String> = keys.iter().flatten().map(ToString::to_string).collect();
    Some(OverBudget {
        code,
        budget,
        over: Over::Batch {
            unlisted: keys.len().saturating_sub(named.len()),
            left_out: named,
            from: value.left_out_from(stop),
            fetch: rerun::fetch_all(value, rerun),
        },
    })
}

/// Writes the note of the rows that report a problem, when `problems` is
/// more than 0, and the tool's note to `err` in one write. A failed write
/// to `err` goes unreported, because standard error is the last place to
/// report anything.
fn write_notes<T: Rows>(err: &mut impl Write, value: &T, problems: usize) {
    let mut notes = Vec::new();
    if problems > 0 {
        notes.extend(problems::note(problems, value.description()).as_bytes());
        notes.push(b'\n');
    }
    if let Some(note) = value.note() {
        notes.extend(note.as_bytes());
        notes.push(b'\n');
    }
    if !notes.is_empty() {
        let _ = err.write_all(&notes).and_then(|()| err.flush());
    }
}
