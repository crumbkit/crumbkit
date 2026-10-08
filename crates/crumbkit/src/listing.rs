//! A page of rows with the total number of matching rows.

use serde::{Serialize, Serializer};

use crate::{Row, Rows};

#[cfg(test)]
mod tests;

/// One page of rows, the total number of rows that matched, and an optional
/// phrase that names them.
///
/// A command returns a listing when it may show fewer rows than matched,
/// and flattens [`Page`](crate::Page) into its arguments so the caller can
/// choose how many. [`finish`](crate::finish) prints the total under `total`
/// in JSON. In text it prints a footer after the table when fewer rows are
/// shown than the total, such as `Showing 4 of 6 books. --all shows every
/// one.`
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::Listing;
///
/// let books: Vec<Book> = (1..=6).map(|id| Book { id }).collect();
/// let listing = Listing::from_all(books, Some(4)).described("books");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing<R: Row> {
    /// The rows to print, in order.
    rows: Vec<R>,
    /// The number of rows that matched, which `new` keeps at least
    /// `rows.len()`.
    total: usize,
    /// The phrase that names the rows in the footer, such as `books`.
    description: Option<String>,
}

impl<R: Row> Listing<R> {
    /// Returns a listing of `rows` out of `total` matching rows.
    ///
    /// A handler that fetches one page and counts the matching rows itself,
    /// such as with a `LIMIT` query and a `COUNT` query, calls `new`, so it
    /// never loads a row it will not print.
    ///
    /// A `total` smaller than the number of rows is raised to the number of
    /// rows, so a document never shows fewer matching rows than it holds.
    /// This happens when rows are added between the count and the fetch.
    #[must_use]
    pub const fn new(rows: Vec<R>, total: usize) -> Self {
        // `usize::max` is not a `const fn`, so this compares by hand.
        let total = if total < rows.len() {
            rows.len()
        } else {
            total
        };
        Self {
            rows,
            total,
            description: None,
        }
    }

    /// Returns a listing of the first `limit` of `rows`, with the length of
    /// `rows` as the total. `None` keeps every row.
    ///
    /// It suits a source that already holds every row in memory. It shortens
    /// `rows` in place, so it allocates nothing. A tool with a database or a
    /// remote backend fetches one page and calls [`Listing::new`] instead.
    #[must_use]
    pub fn from_all(mut rows: Vec<R>, limit: Option<usize>) -> Self {
        let total = rows.len();
        if let Some(limit) = limit {
            rows.truncate(limit);
        }
        Self::new(rows, total)
    }

    /// Sets the phrase that names the rows in the text footer, such as
    /// `books` or `books by author 4`. The JSON document leaves it out.
    ///
    /// The phrase is a `String`, because a tool often builds it at runtime
    /// from its arguments. A `Cow<'static, str>` would save one small
    /// allocation per command and make the signature harder to read.
    #[must_use]
    pub fn described(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Serializes the listing as the array of its rows. [`finish`](crate::finish)
/// writes the total beside it, under `total`.
impl<R: Row> Serialize for Listing<R> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<R: Row> Rows for Listing<R> {
    type Row = R;
    const SINGLE: bool = false;

    fn rows(&self) -> &[R] {
        &self.rows
    }

    fn total(&self) -> Option<usize> {
        Some(self.total)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
