//! The paging flags, which choose how many rows a listing prints.

use std::borrow::Cow;
use std::num::NonZeroUsize;

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

#[cfg(test)]
mod tests;

/// The flags that choose how many rows a listing prints.
///
/// A command that returns a [`Listing`](crate::Listing) adds them to its own
/// arguments struct with `#[command(flatten)]`. They belong to that command,
/// so they are not global.
///
/// - `--limit <N>` prints at most `N` rows. Its default is the const
///   parameter `N`, which is 25 unless the command names another size, as in
///   `Page<10>`, and the command's `--help` shows it. clap refuses 0.
/// - `--all` prints every row, and conflicts with `--limit`.
///
/// ```
/// use clap::Parser;
/// use crumbkit::Page;
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// struct BooksList {
///     #[command(flatten)]
///     page: Page<10>,
/// }
///
/// let args = BooksList::parse_from(["books-list"]);
/// assert_eq!(args.page.limit(), Some(10));
///
/// let args = BooksList::parse_from(["books-list", "--all"]);
/// assert_eq!(args.page.limit(), None);
/// ```
///
/// `N` is at least 1, and a command that flattens `Page<0>` fails to
/// build:
///
/// ```compile_fail
/// #[derive(clap::Parser)]
/// struct BooksList {
///     #[command(flatten)]
///     page: crumbkit::Page<0>,
/// }
///
/// fn main() {
///     let _ = <BooksList as clap::Parser>::parse();
/// }
/// ```
#[derive(Args, Clone, Debug, PartialEq, Eq)]
pub struct Page<const N: usize = 25> {
    /// Print at most this many rows
    // clap's `default_value_t` keeps the default text in one `static` that
    // every `Page<N>` shares, so two commands with different sizes would
    // show the same default. `default_value` with an owned `String`, which
    // clap's `string` feature allows, gives each `N` its own default. The
    // field is a `NonZeroUsize`, so clap refuses 0 when it parses the value.
    #[arg(long, value_name = "N", default_value = Self::DEFAULT_LIMIT.to_string())]
    limit: NonZeroUsize,

    /// Print every row
    #[arg(long, conflicts_with = "limit")]
    all: bool,
}

impl<const N: usize> Page<N> {
    /// The default of `--limit`, which is `N`.
    ///
    /// clap refuses a limit of 0, so `Page<0>` would refuse every call that
    /// leaves out `--limit`. The assertion turns that into a build error,
    /// because the compiler evaluates this constant when it generates the
    /// code that adds the flags to a command (RUST-5).
    const DEFAULT_LIMIT: usize = {
        assert!(N >= 1, "a Page's default limit is at least 1");
        N
    };

    /// Returns the number of rows to print, or `None` when `--all` asks for
    /// every row.
    #[must_use]
    pub const fn limit(&self) -> Option<usize> {
        if self.all {
            None
        } else {
            Some(self.limit.get())
        }
    }
}

/// Serializes the flags as an object with `limit` and `all`, such as
/// `{"limit":25,"all":false}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`. An
/// arguments struct that flattens the page into its clap arguments flattens
/// it with `#[serde(flatten)]` too, so its fields sit beside the command's
/// own.
impl<const N: usize> Serialize for Page<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut page = serializer.serialize_map(Some(2))?;
        self.serialize_fields(&mut page)?;
        page.end()
    }
}

impl<const N: usize> Page<N> {
    /// Writes `limit` and `all` as two entries of `map`. `Page`'s own
    /// `Serialize` writes them, and so does an arguments type in crumbkit
    /// that flattens the page beside its own fields, because crumbkit has no
    /// `serde_derive` for `#[serde(flatten)]`.
    pub(crate) fn serialize_fields<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
        map.serialize_entry("limit", &self.limit)?;
        map.serialize_entry("all", &self.all)
    }
}

/// Describes the object that `Serialize` writes, with `N` as the default of
/// `limit`.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. The name and the id include `N`, because the default differs
/// for each `N`.
impl<const N: usize> JsonSchema for Page<N> {
    fn schema_name() -> Cow<'static, str> {
        format!("Page_{N}").into()
    }

    fn schema_id() -> Cow<'static, str> {
        format!("crumbkit::Page<{N}>").into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "limit": {
                    "description": "Print at most this many rows.",
                    "type": "integer",
                    "minimum": 1,
                    "default": N,
                },
                "all": {
                    "description": "Print every row.",
                    "type": "boolean",
                    "default": false,
                },
            },
            "required": ["limit", "all"],
        })
    }
}
