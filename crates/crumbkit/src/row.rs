//! The traits that let a result print as a table.

use schemars::JsonSchema;
use serde::Serialize;

/// A type that prints as one table row.
///
/// The names in `COLUMNS` and `LONG_COLUMNS` are the field names as the type
/// serializes them, after any `#[serde(rename)]`. A name that is not a field
/// prints `-` in every row of its column, and `null` in JSON, so check each
/// name against the type's fields.
///
/// ```
/// use crumbkit::Row;
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Book {
///     id: u32,
///     title: String,
///     pages: u32,
///     isbn: String,
/// }
///
/// impl Row for Book {
///     const COLUMNS: &'static [&'static str] = &["id", "title"];
///     const LONG_COLUMNS: &'static [&'static str] = &["pages"];
/// }
/// ```
///
/// Here `isbn` is in neither list, so it appears only with `--json --full`.
pub trait Row: Serialize + JsonSchema {
    /// The columns that the compact output shows, in display order.
    const COLUMNS: &'static [&'static str];

    /// The columns that `--long` adds after `COLUMNS`, in display order. A
    /// field in neither list appears only with `--json --full`.
    const LONG_COLUMNS: &'static [&'static str] = &[];
}

/// A result that prints as a table: one [`Row`], or a `Vec` of rows.
///
/// crumbkit implements this trait, and a tool cannot implement it, so crumbkit
/// can add result shapes later without breaking a tool.
pub trait Rows: Serialize + sealed::Sealed {
    /// The type of each row.
    type Row: Row;

    /// Whether the result is a single row. A single row's JSON `data` is an
    /// object, and a list's is an array.
    const SINGLE: bool;

    /// Returns the rows in order. A single row is a slice of one.
    fn rows(&self) -> &[Self::Row];
}

impl<R: Row> Rows for R {
    type Row = R;
    const SINGLE: bool = true;

    fn rows(&self) -> &[R] {
        std::slice::from_ref(self)
    }
}

impl<R: Row> Rows for Vec<R> {
    type Row = R;
    const SINGLE: bool = false;

    fn rows(&self) -> &[R] {
        self
    }
}

/// Holds the supertrait that keeps tools from implementing [`Rows`].
mod sealed {
    /// A type that crumbkit implements [`Rows`](super::Rows) for. The trait is
    /// public so it can bound `Rows`, and it sits in a private module so no
    /// tool can name it.
    pub trait Sealed {}

    impl<R: super::Row> Sealed for R {}

    impl<R: super::Row> Sealed for Vec<R> {}
}
