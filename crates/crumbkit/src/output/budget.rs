//! The output budget: the most bytes that a run prints on standard output,
//! the variable that lowers it, the writer that measures a result before it
//! prints, and the failure of a result that does not fit (ADR-0046).

use std::ffi::OsStr;
use std::fmt;
use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::{Call, Code, Failure, Field, FieldValue, Recovery};

#[cfg(test)]
mod tests;

/// The output budget of a tool that sets none: 1 MiB.
pub(crate) const DEFAULT: usize = 1_048_576;

/// The smallest budget that a tool can set with `Guide::output_budget`.
pub(crate) const SMALLEST: usize = 1_024;

/// The environment variable that lowers the budget of one run, which
/// crumbkit-test's `check_output_budget` sets.
pub(crate) const VARIABLE: &str = "CRUMBKIT_OUTPUT_BUDGET";

/// Returns the budget of a run of a tool whose budget is `budget`, after the
/// value of [`VARIABLE`], which lowers it and never raises it.
pub(crate) fn from_environment(budget: usize) -> usize {
    #[expect(
        clippy::disallowed_methods,
        reason = "the output budget owns the variable that lowers it"
    )]
    let value = std::env::var_os(VARIABLE);
    lowered(budget, value.as_deref())
}

/// Returns `budget` lowered to `value` when `value` is a whole number of
/// bytes from 1 up to `budget`. Any other value, such as a larger number, 0
/// or text, leaves `budget` unchanged, so a run can never print more than
/// the tool allows.
pub(crate) fn lowered(budget: usize, value: Option<&OsStr>) -> usize {
    let Some(bytes) = value
        .and_then(OsStr::to_str)
        .and_then(|text| text.parse::<usize>().ok())
    else {
        return budget;
    };
    if bytes == 0 {
        budget
    } else {
        bytes.min(budget)
    }
}

/// A writer that keeps nothing and counts the bytes it receives, so `finish`
/// learns the size of a result with fewer rows without allocating.
#[derive(Debug, Default)]
pub(super) struct Counter {
    /// The bytes received so far.
    pub(super) bytes: usize,
}

impl Write for Counter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes = self.bytes.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns the number of bytes that `write` writes.
pub(super) fn measure(write: impl FnOnce(&mut Counter) -> io::Result<()>) -> io::Result<usize> {
    let mut counter = Counter::default();
    write(&mut counter)?;
    Ok(counter.bytes)
}

/// The room that a [`Buffer`] starts with when the budget is larger: 16 KiB,
/// which holds most results, so most results allocate one buffer.
const START: usize = 16_384;

/// A writer that keeps the bytes it receives up to a budget, so `finish`
/// serializes a result that fits once and then prints it. A write that
/// would pass the budget fails, which stops the serialization.
#[derive(Debug)]
pub(super) struct Buffer {
    /// The bytes received so far.
    bytes: Vec<u8>,
    /// The most bytes that the buffer keeps.
    budget: usize,
    /// Whether a write would have passed the budget.
    over: bool,
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > self.budget - self.bytes.len() {
            self.over = true;
            return Err(io::Error::other("the result passes the output budget"));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns the bytes that `write` writes, or `None` when they pass
/// `budget`. The buffer never holds more than `budget` bytes.
pub(super) fn buffered(
    budget: usize,
    write: impl FnOnce(&mut Buffer) -> io::Result<()>,
) -> io::Result<Option<Vec<u8>>> {
    let mut buffer = Buffer {
        bytes: Vec::with_capacity(budget.min(START)),
        budget,
        over: false,
    };
    let written = write(&mut buffer);
    if buffer.over {
        return Ok(None);
    }
    written?;
    Ok(Some(buffer.bytes))
}

/// Returns the largest count from `low` to `high` for which `fits` is true,
/// or `low` when none is. `fits` must be true for every count below one for
/// which it is true, which holds because a result with more rows is never
/// shorter, so a binary search finds the count.
pub(super) fn largest(
    low: usize,
    high: usize,
    mut fits: impl FnMut(usize) -> io::Result<bool>,
) -> io::Result<usize> {
    let mut found = low;
    let (mut from, mut to) = (low, high);
    while from <= to {
        let middle = from + (to - from) / 2;
        if fits(middle)? {
            found = middle;
            from = middle + 1;
        } else if middle == 0 {
            break;
        } else {
            to = middle - 1;
        }
    }
    Ok(found)
}

/// The mark of a listing or a list that the budget cut short, which its
/// JSON document holds under `cut`: the over-budget code in the tool's
/// spelling and the bytes that the result had.
#[derive(Debug)]
pub(super) struct Cut {
    /// The over-budget code in the tool's spelling.
    pub(super) code: String,
    /// The bytes that the result had to fit in.
    pub(super) budget: usize,
}

impl Serialize for Cut {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("budget", &self.budget)?;
        map.end()
    }
}

/// Writes the line that ends the text of a result that the budget cut
/// short, such as `Showing 812 of 5000 books, because output stops at
/// 1048576 bytes. Ask for less with --limit or --fields.` A listing without
/// a `total` gives no count, as in `Showing 812 books, because ...`. A
/// listing names `--limit`, and a list, which has no paging flags, names
/// only `--fields`.
pub(super) fn write_cut_line(
    out: &mut impl Write,
    shown: usize,
    total: Option<usize>,
    description: Option<&str>,
    budget: usize,
    listing: bool,
) -> io::Result<()> {
    super::listing::write_shown(out, shown, total, description)?;
    let flags = if listing {
        "--limit or --fields"
    } else {
        "--fields"
    };
    writeln!(
        out,
        ", because output stops at {budget} bytes. Ask for less with {flags}."
    )
}

/// The kind of result that passed the budget, which decides the failure's
/// message, fields and recovery.
#[derive(Clone, Debug)]
pub(super) enum Over {
    /// A batch whose later answers were left out.
    Batch {
        /// The keys of the answers left out that the failure names, in the
        /// order the caller gave them.
        left_out: Vec<String>,
        /// The number of answers left out that the failure does not name:
        /// those without a key, and those whose keys did not fit.
        unlisted: usize,
        /// The call that fetches the keys left out, one at a time.
        fetch: Option<Call>,
    },
    /// A single row or a list of rows that crumbkit refused whole.
    Row,
    /// A prose or bytes result, which no flag makes smaller.
    Whole,
    /// A stream that stopped before the line that would pass the budget.
    Stream,
}

/// The failure of a result that passed the output budget.
#[derive(Debug)]
pub(super) struct OverBudget {
    /// The over-budget code: the document's own, or crumbkit's.
    pub(super) code: &'static Code,
    /// The bytes that the result had to fit in.
    pub(super) budget: usize,
    /// What passed the budget.
    pub(super) over: Over,
}

impl fmt::Display for OverBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let budget = self.budget;
        match &self.over {
            Over::Batch {
                left_out, unlisted, ..
            } => match left_out.len().saturating_add(*unlisted) {
                1 => write!(
                    f,
                    "1 more answer did not fit in the output budget of {budget} bytes"
                ),
                count => write!(
                    f,
                    "{count} more answers did not fit in the output budget of {budget} bytes"
                ),
            },
            Over::Row | Over::Whole => write!(
                f,
                "the result is larger than the output budget of {budget} bytes"
            ),
            Over::Stream => write!(
                f,
                "the stream stopped at the output budget of {budget} bytes"
            ),
        }
    }
}

impl std::error::Error for OverBudget {}

impl Failure for OverBudget {
    fn code(&self) -> &'static Code {
        self.code
    }

    fn recovery(&self) -> Recovery {
        match &self.over {
            Over::Batch {
                fetch: Some(call), ..
            } => Recovery::Run(call.clone()),
            Over::Batch { fetch: None, .. } => Recovery::Hint(
                "run the command again with the keys that were not answered".to_owned(),
            ),
            Over::Row => Recovery::Hint("ask for fewer fields with --fields".to_owned()),
            Over::Whole => Recovery::None {
                reason: "no flag makes this answer smaller".to_owned(),
            },
            Over::Stream => {
                Recovery::Hint("ask for fewer rows, or for fewer fields with --fields".to_owned())
            }
        }
    }

    fn fields(&self) -> Vec<Field<'_>> {
        let budget = Field::new("budget", self.budget);
        match &self.over {
            Over::Batch {
                left_out, unlisted, ..
            } => {
                let mut fields = vec![Field::new("left_out", FieldValue::list(left_out))];
                if *unlisted > 0 {
                    fields.push(Field::new("unlisted", *unlisted));
                }
                fields.push(budget);
                fields
            }
            Over::Row | Over::Whole | Over::Stream => vec![budget],
        }
    }
}
