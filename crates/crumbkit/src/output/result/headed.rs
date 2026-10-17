//! How the text of a batch whose document is headed prints: each answer's
//! bytes under a `==> KEY <==` header, within the output budget (ADR-0062).

use std::borrow::Cow;
use std::io::{self, Write};

use super::{Answer, per_key};
use crate::output::budget::{Over, OverBudget};
use crate::output::table::escape_controls;
use crate::output::{Output, layout, problems};
use crate::{Code, Document, Rows, codes};

/// Writes the rows of `value` to `out` in the text of a headed document,
/// within the bytes that [`Output::budget`] has left, then flushes `out`.
/// `bytes_of` returns the bytes of the row at a position, or `None` for a
/// row that holds none. It returns what it printed and the number of bytes
/// it wrote.
///
/// Each row prints whole or not at all. A row prints its bytes unchanged,
/// or one line of compact JSON with every field when it holds no bytes.
/// When the batch has more than one key, a header line `==> KEY <==`
/// precedes each row, with the key's control characters escaped, and a
/// line feed precedes each header after the first, as `head` prints many
/// files. A key's failure prints no header.
///
/// A row whose header and content alone pass the budget does not print,
/// and gets its own over-budget failure under its key, unless it is the
/// first row of a document that always prints its first answer. The other
/// rows print in order while they fit. From the first row that does not
/// fit, every later row and every key the batch never fetched is left out,
/// and one failure names them, or each gets a failure of its own when the
/// document answers each key left out (ADR-0068). After `out` is flushed, the note of the rows
/// that report a problem and the tool's note go to `err`, as they do for a
/// table.
pub(super) fn write<T: Rows>(
    out: &mut impl Write,
    err: &mut impl Write,
    output: &Output,
    document: Document<T>,
    value: &T,
    bytes_of: fn(&T, usize) -> Option<&[u8]>,
    problems: usize,
) -> io::Result<(Answer, usize)> {
    let budget = output.budget();
    let code = output.renames.printed(&codes::OUTPUT_OVER_BUDGET);
    let rows = value.rows();
    let headers = crate::output::several_keys(value) || rows.len() > 1;
    let mut oversize = Vec::new();
    let mut printed: Vec<(String, Cow<'_, [u8]>)> = Vec::new();
    let mut spent = 0_usize;
    let mut stop = rows.len();
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
            layout::serialize(&mut line, row, output.json_style.keys)?;
            line.push(b'\n');
            Cow::Owned(line)
        };
        let alone = header.len().saturating_add(content.len());
        let first = index == 0 && document.always_prints_first();
        if alone > budget && !first {
            let over = OverBudget {
                code,
                budget,
                over: Over::Whole,
            };
            oversize.push((position, over));
            continue;
        }
        let separator = usize::from(!printed.is_empty());
        let cost = alone.saturating_add(separator);
        if spent.saturating_add(cost) > budget && !first {
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
    // The rows from `stop` on come after every row too large alone, so the
    // failures stay in key order.
    let left_out = if document.answers_left_out_per_key() {
        oversize.extend(per_key::each_left_out(value, stop, code, budget));
        None
    } else {
        left_out(value, stop, code, budget)
    };
    if oversize.is_empty() && left_out.is_none() {
        write_notes(err, value, problems);
    }
    let answer = Answer::Keyed {
        each: oversize,
        left_out,
    };
    Ok((answer, spent))
}

/// Returns the header line of the row at `position`, whose key is `key`:
/// `==> KEY <==` with the key's control characters escaped, or the
/// position in parentheses for a row pushed after the last key.
pub(in crate::output) fn header(key: Option<&str>, position: usize) -> String {
    match key {
        Some(key) => format!("==> {} <==\n", escape_controls(key)),
        None => format!("==> (index {position}) <==\n"),
    }
}

/// Returns the failure of the rows from the row numbered `stop` on and the
/// keys the batch never fetched, which names every key, or `None` when no
/// row or key is left out.
fn left_out<T: Rows>(
    value: &T,
    stop: usize,
    code: &'static Code,
    budget: usize,
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
            fetch: value.fetch(),
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
