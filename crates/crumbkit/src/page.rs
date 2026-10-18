//! The paging flags, which choose how many rows a listing prints.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::num::{IntErrorKind, NonZeroUsize};

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::{Listing, Row};

#[cfg(test)]
mod tests;

/// The help text of `--limit`, which the schema also uses as its description.
const LIMIT_HELP: &str = "Print at most this many rows";

/// The help text of `--all`, which the schema also uses as its description.
const ALL_HELP: &str = "Print every row";

/// The flags that choose how many rows a listing prints.
///
/// A command that returns a [`Listing`](crate::Listing) adds them to its own
/// arguments struct with `#[command(flatten)]`. They belong to that command,
/// so they are not global.
///
/// - `--limit <N>` prints at most `N` rows. Its default is the const
///   parameter `N`, which is 25 unless the command names another size, as in
///   `Page<10>`, and the command's `--help` shows it. clap refuses 0, and
///   the refusal states the range that `--limit` takes.
/// - `--all` prints every row, and conflicts with `--limit`.
///
/// The second const parameter `MOST` is the most rows that `--limit` takes.
/// It has no bound unless the command names one, as in `Page<10, 100>`. A
/// larger value is refused before the handler runs, with a recovery that
/// runs the caller's command with `--all` in place of `--limit`. The help
/// of `--limit` and the footer of a listing that the page builds with
/// [`Page::first_of`] or [`Page::listing`] name the bound (ADR-0050).
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
/// `N` is at least 1 and at most `MOST`, and a command that flattens
/// `Page<0>` or `Page<10, 5>` fails to build:
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
///
/// ```compile_fail
/// #[derive(clap::Parser)]
/// struct BooksList {
///     #[command(flatten)]
///     page: crumbkit::Page<10, 5>,
/// }
///
/// fn main() {
///     let _ = <BooksList as clap::Parser>::parse();
/// }
/// ```
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct Page<const N: usize = 25, const MOST: usize = { usize::MAX }> {
    // clap's `default_value_t` keeps the default text in one `static` that
    // every `Page<N>` shares, so two commands with different sizes would
    // show the same default. `default_value` with an owned `String`, which
    // clap's `string` feature allows, gives each `N` its own default. The
    // field is a `NonZeroUsize`, and `parse_limit` refuses 0 and any value
    // above `MOST` when clap parses the value.
    #[arg(
        long,
        value_name = "N",
        help = Self::limit_help(),
        default_value = Self::DEFAULT_LIMIT.to_string(),
        value_parser = Self::parse_limit,
    )]
    limit: NonZeroUsize,

    #[arg(long, help = ALL_HELP, conflicts_with = "limit")]
    all: bool,
}

impl<const N: usize, const MOST: usize> Page<N, MOST> {
    /// The default of `--limit`, which is `N`.
    ///
    /// clap refuses a limit of 0 and a limit above `MOST`, so `Page<0>` or
    /// `Page<10, 5>` would refuse every call that leaves out `--limit`. The
    /// assertions turn that into a build error, because the compiler
    /// evaluates this constant when it generates the code that adds the
    /// flags to a command (RUST-5).
    const DEFAULT_LIMIT: usize = {
        assert!(N >= 1, "a Page's default limit is at least 1");
        assert!(N <= MOST, "a Page's default limit is at most its bound");
        N
    };

    /// Returns the help text of `--limit`, which names the bound when the
    /// page has one, such as `Print at most this many rows, up to 100`.
    fn limit_help() -> String {
        match Self::bound() {
            Some(most) => format!("{LIMIT_HELP}, up to {most}"),
            None => LIMIT_HELP.to_owned(),
        }
    }

    /// Reads the value of `--limit`: a whole number from 1 up to `MOST`. A
    /// larger number on a page with a bound, also one too large for
    /// `usize`, fails with [`LimitAbove`], which `parse` reads to offer
    /// `--all` in its recovery. Every other refused value fails with the
    /// same sentence, which states the range that `--limit` takes.
    fn parse_limit(text: &str) -> Result<NonZeroUsize, Box<dyn Error + Send + Sync>> {
        match text.parse::<NonZeroUsize>() {
            Ok(limit) if limit.get() <= MOST => Ok(limit),
            Ok(_) => Err(Box::new(LimitAbove { most: MOST })),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => match Self::bound() {
                Some(most) => Err(Box::new(LimitAbove { most })),
                None => Err(limit_range(Some(MOST)).into()),
            },
            Err(_) => Err(limit_range(Self::bound()).into()),
        }
    }

    /// Returns `MOST`, or `None` when `--limit` has no bound.
    const fn bound() -> Option<usize> {
        if MOST == usize::MAX { None } else { Some(MOST) }
    }

    /// Returns the most rows that `--limit` takes, or `None` when it has no
    /// bound.
    #[must_use]
    pub const fn most(&self) -> Option<usize> {
        Self::bound()
    }

    /// Returns a listing of the first rows of `rows` that the flags ask
    /// for, with the length of `rows` as the total, as
    /// [`Listing::from_all`] does. The listing carries the page's bound, so
    /// its footer names it, and it counts the rows that report a problem
    /// among every row of `rows`, so a limit never hides one from the exit.
    #[must_use]
    pub fn first_of<R: Row>(&self, rows: Vec<R>) -> Listing<R> {
        Self::carry(Listing::from_all(rows, self.limit()))
    }

    /// Returns a listing of `rows` out of `total` matching rows, as
    /// [`Listing::new`] does, for a handler that fetched one page and
    /// counted the matching rows itself. The listing carries the page's
    /// bound, so its footer names it.
    #[must_use]
    pub fn listing<R: Row>(&self, rows: Vec<R>, total: usize) -> Listing<R> {
        Self::carry(Listing::new(rows, total))
    }

    /// Returns a listing of the first rows of `rows` that the flags ask for,
    /// for a source that the tool does not count, as
    /// [`Listing::uncounted`] does. The listing carries the page's bound, so
    /// its footer names it.
    ///
    /// It reads at most one row past the limit, and that row tells it
    /// whether more rows exist, so an iterator over a log or a remote API
    /// stops there. With `--all` it reads every row. The listing has no
    /// total, even when it holds every row of the source. A
    /// handler that fetches a `Vec` fetches one row past [`Page::limit`],
    /// because a `Vec` of exactly the limit reads as a source with no more
    /// rows.
    #[must_use]
    pub fn uncounted<R: Row>(&self, rows: impl IntoIterator<Item = R>) -> Listing<R> {
        let Some(limit) = self.limit() else {
            return Self::carry(Listing::uncounted(rows.into_iter().collect(), false));
        };
        // `take` on a `Vec`'s iterator collects in place, so a `Vec` is not
        // copied.
        let mut rows: Vec<R> = rows.into_iter().take(limit.saturating_add(1)).collect();
        let more = rows.len() > limit;
        rows.truncate(limit);
        Self::carry(Listing::uncounted(rows, more))
    }

    /// Returns `listing` with the page's bound, when it has one.
    const fn carry<R: Row>(listing: Listing<R>) -> Listing<R> {
        match Self::bound() {
            Some(most) => listing.bounded(most),
            None => listing,
        }
    }

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
impl<const N: usize, const MOST: usize> Serialize for Page<N, MOST> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut page = serializer.serialize_map(Some(2))?;
        self.serialize_fields(&mut page)?;
        page.end()
    }
}

impl<const N: usize, const MOST: usize> Page<N, MOST> {
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
/// `limit` and `MOST`, when the page has a bound, as its maximum.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. The name and the id include `N`, and `MOST` when the page has a
/// bound, because the default and the maximum differ for each.
impl<const N: usize, const MOST: usize> JsonSchema for Page<N, MOST> {
    fn schema_name() -> Cow<'static, str> {
        match Self::bound() {
            Some(most) => format!("Page_{N}_{most}").into(),
            None => format!("Page_{N}").into(),
        }
    }

    fn schema_id() -> Cow<'static, str> {
        match Self::bound() {
            Some(most) => format!("crumbkit::Page<{N}, {most}>").into(),
            None => format!("crumbkit::Page<{N}>").into(),
        }
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        let mut limit = json_schema!({
            "description": Self::limit_help(),
            "type": "integer",
            "minimum": 1,
            "default": N,
        });
        if let Some(most) = Self::bound() {
            limit.insert("maximum".to_owned(), most.into());
        }
        json_schema!({
            "type": "object",
            "properties": {
                "limit": limit,
                "all": {
                    "description": ALL_HELP,
                    "type": "boolean",
                    "default": false,
                },
            },
            "required": ["limit", "all"],
        })
    }
}

/// The refusal of a `--limit` above the page's bound, which `parse` reads
/// from clap's error to offer `--all` and `--limit MOST` in its recovery.
#[derive(Debug)]
pub(crate) struct LimitAbove {
    /// The most rows that `--limit` takes.
    pub(crate) most: usize,
}

impl fmt::Display for LimitAbove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&limit_range(Some(self.most)))
    }
}

/// Returns the sentence that refuses a value of `--limit`, which states the
/// range it takes: from 1 to `most`, or at least 1 when `most` is `None`.
fn limit_range(most: Option<usize>) -> String {
    match most {
        Some(most) => format!("--limit takes a whole number from 1 to {most}"),
        None => "--limit takes a whole number of at least 1".to_owned(),
    }
}

impl Error for LimitAbove {}
