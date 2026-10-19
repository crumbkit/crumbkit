//! A page of rows with the total number of matching rows, when the tool
//! counted them.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{Call, Row, Rows, Shape};

#[cfg(test)]
mod tests;

/// One page of rows, the total number of rows that matched when the tool
/// counted them, an optional phrase that names them, an optional summary, an optional note and an
/// optional next step.
///
/// A command returns a listing when it may show fewer rows than matched,
/// and flattens `crumbkit::Page` into its arguments so the caller can
/// choose how many. `crumbkit::finish` prints the total under `total`
/// in JSON, and `truncated`, which is true when the document leaves out
/// rows. In text it prints the table on standard output, and then, on
/// standard error, a footer when fewer rows are shown than the total, such
/// as `Showing 4 of 6 books. --all shows every one.`, or `Showing 4 books,
/// and more match. --all shows every one.` for a listing without a total,
/// then the note, and
/// then the next step, such as `Next: shelf books show <ID>... takes up to
/// 100 ids in one call.`
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Call, Listing};
///
/// let books: Vec<Book> = (1..=6).map(|id| Book { id }).collect();
/// let listing = Listing::from_all(books, Some(4))
///     .described("books")
///     .next(Call::new(&["books", "show"]).arg("ids"), "takes up to 100 ids in one call.");
/// ```
///
/// The summary `S` is a value that prints as one line above the table in
/// text and as an object under `summary` in JSON. A listing without one
/// has the summary type [`NoSummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing<R: Row, S = NoSummary> {
    /// The rows to print, in order.
    rows: Vec<R>,
    /// The number of rows that matched, which `new` keeps at least
    /// `rows.len()`, or `None` when the tool did not count them.
    total: Option<usize>,
    /// Whether the source of a listing without a total holds rows after
    /// these. A listing with a total reads this from the total instead.
    more: bool,
    /// The phrase that names the rows in the footer, such as `books`.
    description: Option<String>,
    /// The summary that the text output prints above the table and the
    /// JSON document holds under `summary`.
    summary: Option<S>,
    /// The line that the text output prints after the table and any
    /// footer, such as the path of the file that the rows come from.
    note: Option<String>,
    /// The command that the text output names as the next step, and the
    /// phrase that follows it on the `Next:` line.
    next: Option<(Call, String)>,
    /// The most rows that the command's `--limit` takes, which the footer
    /// names, or `None` when `--limit` has no upper bound.
    bound: Option<usize>,
    /// The number of rows that report a problem among every row that
    /// [`Listing::from_all`] was given, when its limit cut some of them and
    /// the row type names a problem field, or `None` when the listing counts
    /// the rows it holds.
    problems: Option<usize>,
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
            total: Some(total),
            more: false,
            description: None,
            summary: None,
            note: None,
            next: None,
            bound: None,
            problems: None,
        }
    }

    /// Returns a listing of `rows`, one page of a source that the tool did
    /// not count, such as a remote API that pages its answers or a log that
    /// grows while it is read. `more` says whether the source holds rows
    /// after these.
    ///
    /// The listing never has a total, whether or not more rows exist, so
    /// its JSON document leaves out `total`. With more rows, its footer
    /// reads `Showing 4 books, and more match. --all shows every one.` and
    /// its document holds `"truncated": true`. Without more rows, it holds
    /// every row of the source and prints no footer.
    ///
    /// A handler learns `more` by fetching one row past the page, which
    /// `crumbkit::Page::uncounted` does, or from a source that says whether
    /// another page exists.
    #[must_use]
    pub const fn uncounted(rows: Vec<R>, more: bool) -> Self {
        let mut listing = Self::new(rows, 0);
        listing.total = None;
        listing.more = more;
        listing
    }

    /// Returns a listing of the first `limit` of `rows`, with the length of
    /// `rows` as the total. `None` keeps every row.
    ///
    /// It suits a source that already holds every row in memory. It shortens
    /// `rows` in place, so it allocates nothing. A tool with a database or a
    /// remote backend fetches one page and calls [`Listing::new`] instead.
    ///
    /// When the limit cuts rows and the row type names a problem field
    /// ([`Row::PROBLEM`]), it counts the rows that report a problem among
    /// every row before it cuts them, so `crumbkit::finish` exits with the
    /// problem exit and its note counts every such row although the page
    /// holds fewer. A listing from [`Listing::new`] or
    /// [`Listing::uncounted`] holds one page that the tool fetched, so
    /// crumbkit counts only the rows of that page.
    #[must_use]
    pub fn from_all(mut rows: Vec<R>, limit: Option<usize>) -> Self {
        let total = rows.len();
        let problems = match limit {
            Some(limit) if limit < total && R::PROBLEM.is_some() => {
                Some(crate::row::problems::count(&rows))
            }
            _ => None,
        };
        if let Some(limit) = limit {
            rows.truncate(limit);
        }
        let mut listing = Self::new(rows, total);
        listing.problems = problems;
        listing
    }
}

impl<R: Row, S> Listing<R, S> {
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

    /// Sets the summary: a value that the text output prints with its
    /// `Display` as one line above the table, and that the JSON document
    /// holds under `summary`, serialized with its `Serialize` and described
    /// by its `JsonSchema`. A summary
    /// holds what describes the whole result, such as the figures the rows
    /// were counted from, so a program reads it as fields and a person reads
    /// it as a sentence.
    #[must_use]
    pub fn summary<T: Serialize + fmt::Display + JsonSchema>(self, summary: T) -> Listing<R, T> {
        Listing {
            rows: self.rows,
            total: self.total,
            more: self.more,
            description: self.description,
            summary: Some(summary),
            note: self.note,
            next: self.next,
            bound: self.bound,
            problems: self.problems,
        }
    }

    /// Sets the note: a line that the text output prints on standard error
    /// after any footer and before any next step, such as `Aliases file:
    /// /home/ada/.config/shelf/aliases.toml`. The text carries its own full
    /// stop when it needs one. The JSON document leaves it out, so a note
    /// holds only what a person reading the table needs and a program can do
    /// without.
    #[must_use]
    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    /// Sets the next step that the text output names on standard error after
    /// the footer and the note: the line `Next: {call} {phrase}`, such as
    /// `Next: shelf books show <ID>... takes up to 100 ids in one call.` The
    /// JSON document leaves it out.
    ///
    /// `crumbkit::finish` renders `call` against the command that the run's
    /// `crumbkit::Output` holds, and leaves the line out when `call` does
    /// not resolve. The phrase prints unchanged, so it carries its own full stop.
    #[must_use]
    pub fn next(mut self, call: Call, phrase: impl Into<String>) -> Self {
        self.next = Some((call, phrase.into()));
        self
    }
}

impl<R: Row, S> Listing<R, S> {
    /// Sets the most rows that the command's `--limit` takes, which the text
    /// footer names, such as `Showing 4 of 6 books. --limit takes up to 100,
    /// and --all shows every one.` The JSON document leaves it out.
    ///
    /// `crumbkit::Page::first_of` and `crumbkit::Page::listing` set it from
    /// the page's bound, so a handler that builds its listing from the page
    /// never calls it.
    #[must_use]
    pub const fn bounded(mut self, most: usize) -> Self {
        self.bound = Some(most);
        self
    }
}

/// Serializes the listing as the array of its rows. `crumbkit::finish`
/// writes the total beside it, under `total`.
impl<R: Row, M> Serialize for Listing<R, M> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<R: Row, S: Serialize + fmt::Display + JsonSchema> Rows for Listing<R, S> {
    type Row = R;
    type Summary = S;
    const SHAPE: Shape = Shape::Listing;

    fn rows(&self) -> &[R] {
        &self.rows
    }

    fn total(&self) -> Option<usize> {
        self.total
    }

    fn more(&self) -> bool {
        self.total
            .map_or(self.more, |total| self.rows.len() < total)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn summary(&self) -> Option<&S> {
        self.summary.as_ref()
    }

    fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    fn next_step(&self) -> Option<(&Call, &str)> {
        self.next
            .as_ref()
            .map(|(call, phrase)| (call, phrase.as_str()))
    }

    fn limit_bound(&self) -> Option<usize> {
        self.bound
    }

    fn problems(&self) -> usize {
        self.problems
            .unwrap_or_else(|| crate::row::problems::count(&self.rows))
    }
}

/// The summary type of a listing without a summary. It has no values, so a
/// listing of this type never holds a summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoSummary {}

impl Serialize for NoSummary {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        match *self {}
    }
}

impl fmt::Display for NoSummary {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

/// Describes `NoSummary` as the schema `false`, which no value matches,
/// because a listing of this type never holds a summary. The schema is
/// inline, so a generator returns `false` itself and adds no definition,
/// and crumbkit's document schema leaves out a `summary` whose schema is
/// `false`.
impl JsonSchema for NoSummary {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "NoSummary".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::from(false)
    }
}
