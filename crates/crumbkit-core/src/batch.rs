//! The result of a command that takes many ids.

use std::fmt;

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
/// The handler may stop before the last key, when `crumbkit::Room::has_room`
/// returns false. The keys after the last push were never fetched, and
/// `crumbkit::finish` ends the run with an over-budget failure that names
/// them in its `left_out` field beside the keys of any rows that did not
/// fit, or with one such failure for each of those keys when the document
/// declares `Document::left_out_per_key`. A result pushed after the last key
/// has no key.
///
/// `crumbkit::finish` prints the rows as a `Vec` of rows prints, and
/// then each failure on standard error as it prints a failed command, with
/// a `key:` line that names the key it failed on and the key's index. A
/// batch with no rows prints nothing on standard output in text. With
/// `--json`, the document holds the rows under `data` and the failures under
/// `failures`, each with its `key` and `index`. A batch with at least one
/// failure exits with code 1.
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
/// fn show(ids: &[u32]) -> Batch<Book, NoSuchBook> {
///     let mut batch = Batch::new(&["books", "show"], "ids", ids);
///     batch.extend(ids.iter().map(|&id| find(id)));
///     batch
/// }
/// # let _ = show(&[2, 7, 9]);
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
    /// For each failure, the position of its result among the pushed
    /// results, which is the position of its key in `keys`.
    failure_positions: Vec<usize>,
    /// The number of results pushed so far.
    pushed: usize,
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
            failure_positions: Vec::new(),
            pushed: 0,
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
            Err(failure) => {
                self.failures.push(failure);
                self.failure_positions.push(self.pushed);
            }
        }
        self.pushed += 1;
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

    fn failure_indexes(&self) -> &[usize] {
        &self.failure_positions
    }

    fn key(&self, index: usize) -> Option<&str> {
        self.keys.get(index).map(String::as_str)
    }

    fn row_indexes(&self) -> &[usize] {
        &self.positions
    }

    fn left_out_from(&self, shown: usize) -> Option<usize> {
        match self.positions.get(shown) {
            Some(&position) => Some(position),
            None => (self.pushed < self.keys.len()).then_some(self.pushed),
        }
    }

    fn fetch(&self) -> Option<Call> {
        Some(Call::new(self.command).all(self.arg, "left_out"))
    }

    fn left_out_indexes(&self, shown: usize) -> Vec<usize> {
        let cut = self.positions.get(shown..).unwrap_or_default();
        cut.iter()
            .copied()
            .chain(self.pushed..self.keys.len())
            .collect()
    }

    fn fetch_key(&self, key: &str) -> Option<Call> {
        Some(Call::new(self.command).value(self.arg, key))
    }
}
