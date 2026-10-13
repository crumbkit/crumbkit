//! The result of a command that takes many ids.

use std::fmt;
use std::io;

use serde::{Serialize, Serializer};

use crate::{AsFailure, Call, Failure, Row, Rows, Shape};

#[cfg(test)]
mod tests;

/// The result of a command that takes many keys, such as ids: one result
/// for each key, in the order the caller gave the keys, where a result is a
/// row or a failure.
///
/// [`Batch::new`] takes the command, the clap id of its argument that holds
/// the keys, and the keys. The handler then pushes one `Result` per key, in
/// the order of the keys, with [`Batch::push`] or `extend`. A tool whose
/// backend has a batch lookup, such as an SQL `IN` query, pushes the results
/// of that one call in key order. A tool that looks up one key per call
/// runs its lookups with `crumbkit::fetch_each` and extends the batch with
/// them.
///
/// A handler returns `Result<Batch<R, E>, E2>`. The outer `Err` is a failure
/// of the whole command, such as an unreachable server, and each failure
/// inside the batch belongs to one key.
///
/// The handler may stop before the last key, when [`Batch::has_room`]
/// returns false. The keys after the last push were never fetched, and
/// `crumbkit::finish` ends the run with an over-budget failure that names
/// them in its `left_out` field beside the keys of any rows that did not
/// fit. A result pushed after the last key has no key.
///
/// `crumbkit::finish` prints the rows as a `Vec` of rows prints, and
/// then each failure on standard error as it prints a failed command. A
/// batch with no rows prints nothing on standard output in text. With
/// `--json`, the document holds the rows under `data` and the failures under
/// `failures`. A batch with at least one failure exits with code 1.
///
/// ```
/// # use std::fmt;
/// # use crumbkit_core::{Call, Code, Failure, Recovery};
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit_core::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// # const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// # #[derive(Debug)]
/// # struct NoSuchBook(u32);
/// # impl fmt::Display for NoSuchBook {
/// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
/// #         write!(f, "the catalogue has no book {}", self.0)
/// #     }
/// # }
/// # impl std::error::Error for NoSuchBook {}
/// # impl Failure for NoSuchBook {
/// #     fn code(&self) -> &'static Code { &NOT_FOUND }
/// #     fn recovery(&self) -> Recovery { Recovery::Run(Call::new(&["books", "list"])) }
/// # }
/// # fn find(id: u32) -> Result<Book, NoSuchBook> {
/// #     if id == 2 { Ok(Book { id }) } else { Err(NoSuchBook(id)) }
/// # }
/// use crumbkit_core::Batch;
///
/// // `budget` is the number of bytes the run has left, which
/// // `crumbkit::Output::budget` returns.
/// fn show(ids: &[u32], budget: usize) -> Batch<Book, NoSuchBook> {
///     let mut batch = Batch::new(&["books", "show"], "ids", ids);
///     for &id in ids {
///         if !batch.has_room(budget) {
///             break;
///         }
///         batch.push(find(id));
///     }
///     batch
/// }
/// # let _ = show(&[2, 7, 9], usize::MAX);
/// ```
#[derive(Debug)]
pub struct Batch<R: Row, E: AsFailure> {
    /// The command that takes the keys, by the names of its subcommands
    /// from the root.
    command: &'static [&'static str],
    /// The clap id of the command's argument that holds the keys.
    arg: &'static str,
    /// The keys as text, in the order the caller gave them.
    keys: Vec<String>,
    /// The rows, in the order of their keys.
    rows: Vec<R>,
    /// For each row, the position of its result among the pushed results,
    /// which is the position of its key in `keys`.
    positions: Vec<usize>,
    /// The failures, in the order of their keys.
    failures: Vec<E>,
    /// The number of results pushed so far.
    pushed: usize,
    /// The number of rows that `has_room` has measured.
    measured: usize,
    /// The bytes of compact JSON of the measured rows.
    bytes: usize,
}

impl<R: Row, E: AsFailure> Batch<R, E> {
    /// Returns an empty batch of the command at `command`, the names of its
    /// subcommands from the root, whose argument with the clap id `arg`
    /// takes `keys`. Each key is kept as its text, which is how the
    /// over-budget failure names the keys it leaves out. The argument takes
    /// many values, because that failure's recovery passes it every key
    /// left out in one run.
    ///
    /// The function is not `const`, because it writes each key as text.
    #[must_use]
    pub fn new<K: fmt::Display>(
        command: &'static [&'static str],
        arg: &'static str,
        keys: &[K],
    ) -> Self {
        Self {
            command,
            arg,
            keys: keys.iter().map(ToString::to_string).collect(),
            rows: Vec::new(),
            positions: Vec::new(),
            failures: Vec::new(),
            pushed: 0,
            measured: 0,
            bytes: 0,
        }
    }

    /// Adds the result of the next key: a row for a key that worked, or a
    /// failure for a key that did not.
    pub fn push(&mut self, result: Result<R, E>) {
        match result {
            Ok(row) => {
                self.rows.push(row);
                self.positions.push(self.pushed);
            }
            Err(failure) => self.failures.push(failure),
        }
        self.pushed += 1;
    }

    /// Returns whether the rows pushed so far take fewer than `budget`
    /// bytes, so the handler can fetch the next key.
    ///
    /// The method measures each row once, the first time it runs after the
    /// row was pushed, as compact JSON with every field. A row that fails to
    /// serialize counts the bytes written before the error. The measure
    /// leaves out the document around the rows, the commas between them and
    /// the failures, so a batch that stops when this returns false may still
    /// end over the budget, and `crumbkit::finish` then cuts it.
    pub fn has_room(&mut self, budget: usize) -> bool {
        for row in &self.rows[self.measured..] {
            let mut counter = Counter(0);
            // A row that fails to serialize counts what it wrote, and
            // `finish` reports the serialization error when it prints.
            let _ = serde_json::to_writer(&mut counter, row);
            self.bytes = self.bytes.saturating_add(counter.0);
        }
        self.measured = self.rows.len();
        self.bytes < budget
    }
}

/// Adds each result in turn, as [`Batch::push`] does.
impl<R: Row, E: AsFailure> Extend<Result<R, E>> for Batch<R, E> {
    fn extend<T: IntoIterator<Item = Result<R, E>>>(&mut self, results: T) {
        for result in results {
            self.push(result);
        }
    }
}

/// An `io::Write` that counts the bytes written to it and keeps none.
struct Counter(usize);

impl io::Write for Counter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Serializes the batch as the array of its rows.
/// `crumbkit::finish` writes the failures beside it, under
/// `failures`.
impl<R: Row, E: AsFailure> Serialize for Batch<R, E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<R: Row, E: AsFailure> Rows for Batch<R, E> {
    type Row = R;
    const SHAPE: Shape = Shape::Batch;
    type Summary = crate::NoSummary;

    fn rows(&self) -> &[R] {
        &self.rows
    }

    fn failures(&self) -> Option<Vec<&dyn Failure>> {
        Some(self.failures.iter().map(AsFailure::as_failure).collect())
    }

    fn left_out(&self, shown: usize) -> Vec<Option<&str>> {
        let cut = self.positions.get(shown..).unwrap_or_default();
        let unfetched = self.keys.get(self.pushed..).unwrap_or_default();
        cut.iter()
            .map(|&position| self.keys.get(position).map(String::as_str))
            .chain(unfetched.iter().map(|key| Some(key.as_str())))
            .collect()
    }

    fn fetch(&self) -> Option<Call> {
        Some(Call::new(self.command).all(self.arg, "left_out"))
    }
}
