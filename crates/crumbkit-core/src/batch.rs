//! The result of a command that takes many ids.

use serde::{Serialize, Serializer};

use crate::{AsFailure, Failure, Row, Rows, Shape};

#[cfg(test)]
mod tests;

/// The result of a command that takes many ids: the rows for the ids that
/// worked, in the order the caller gave the ids, and a failure for the ids
/// that did not.
///
/// A handler returns `Result<Batch<R, E>, E2>`. The outer `Err` is a failure
/// of the whole command, such as an unreachable server, and the failures
/// inside the batch belong to individual ids. A tool chooses how many ids
/// one failure names: shelf builds one failure that names every missing id.
///
/// A tool whose backend has a batch lookup, such as an SQL `IN` query or an
/// API that takes many ids, builds the batch from that one call. A tool that
/// can look up only one id per call runs its lookups with
/// `crumbkit::fetch_each`.
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
/// # struct NoSuchBooks(Vec<u32>);
/// # impl fmt::Display for NoSuchBooks {
/// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
/// #         write!(f, "the catalogue has no books {:?}", self.0)
/// #     }
/// # }
/// # impl std::error::Error for NoSuchBooks {}
/// # impl Failure for NoSuchBooks {
/// #     fn code(&self) -> &'static Code { &NOT_FOUND }
/// #     fn recovery(&self) -> Recovery { Recovery::Run(Call::new(&["books", "list"])) }
/// # }
/// # fn find(id: u32) -> Result<Book, u32> {
/// #     if id == 2 { Ok(Book { id }) } else { Err(id) }
/// # }
/// use crumbkit_core::Batch;
///
/// // `find` looks up one book and returns the id itself when it is missing.
/// fn show(ids: &[u32]) -> Batch<Book, NoSuchBooks> {
///     let mut found = Vec::new();
///     let mut missing = Vec::new();
///     for &id in ids {
///         match find(id) {
///             Ok(book) => found.push(book),
///             Err(id) => missing.push(id),
///         }
///     }
///     // One failure names every missing id.
///     let failures = if missing.is_empty() {
///         Vec::new()
///     } else {
///         vec![NoSuchBooks(missing)]
///     };
///     Batch::new(found, failures)
/// }
/// # let _ = show(&[2, 7, 9]);
/// ```
#[derive(Debug)]
pub struct Batch<R: Row, E: AsFailure> {
    /// The rows for the ids that worked, in the order the caller gave the
    /// ids.
    rows: Vec<R>,
    /// The failures for the ids that did not work, in the order they print.
    failures: Vec<E>,
}

impl<R: Row, E: AsFailure> Batch<R, E> {
    /// Returns a batch of the rows for the ids that worked and the failures
    /// for the ids that did not. Both lists print in the order given.
    #[must_use]
    pub const fn new(rows: Vec<R>, failures: Vec<E>) -> Self {
        Self { rows, failures }
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
}
