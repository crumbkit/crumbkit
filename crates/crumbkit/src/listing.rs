//! A page of rows with the total number of matching rows, when the tool
//! counted them.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{Call, Row, Rows, Shape};

#[cfg(test)]
mod tests;

/// One page of rows, the total number of rows that matched when the tool
/// counted them, an optional phrase that names them, an optional summary, an optional note, an
/// optional long note and an optional next step.
///
/// A command returns a listing when it may show fewer rows than matched,
/// and flattens `crumbkit::Page` into its arguments so the caller can
/// choose how many. `crumbkit::finish` prints the total under `total`
/// in JSON, and `truncated`, which is true when the document leaves out
/// rows. In text it prints the table on standard output, and then, on
/// standard error, a footer when fewer rows are shown than the total, such
/// as `Showing 4 of 6 books. --all shows every one.`, or `Showing 4 books,
/// and more match. --all shows every one.` for a listing without a total,
/// or `Showing 4 of at least 100 books. --all shows 100 of them.` for a
/// listing from a capped source ([`Listing::capped`]), then the note, under
/// `--long` the long note, and then the next step, such as `Next: shelf
/// books show <ID>... takes up to 100 ids in one call.`
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
/// text and as an object under `summary` in JSON. A summary whose `Display`
/// writes no text prints no line. A listing without one has the summary
/// type [`NoSummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing<R: Row, S = NoSummary> {
    /// The rows to print, in order.
    rows: Vec<R>,
    /// The number of rows that matched, or that the tool read when the
    /// listing is capped, which `new` keeps at least `rows.len()`, or
    /// `None` when the tool did not count them.
    total: Option<usize>,
    /// Whether the source of a listing without a total holds rows after
    /// these. A listing with a total reads this from the total instead.
    more: bool,
    /// Whether the tool read only part of its source, so `total` counts
    /// the rows it read and the source holds more. Only a listing with a
    /// total is capped.
    capped: bool,
    /// The phrase that names the rows in the footer, such as `books`.
    description: Option<String>,
    /// The phrase that names one row in the footer and the cut line of a
    /// listing without a total, such as `book`.
    singular: Option<String>,
    /// The summary that the text output prints above the table and the
    /// JSON document holds under `summary`.
    summary: Option<S>,
    /// The line that the text output prints after the table and any
    /// footer, such as the path of the file that the rows come from.
    note: Option<String>,
    /// The line that the text output prints after the note under `--long`
    /// only, such as how the figures of the rows were measured.
    long_note: Option<String>,
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
            capped: false,
            description: None,
            singular: None,
            summary: None,
            note: None,
            long_note: None,
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
    /// When the limit cuts rows, it counts the rows that report a problem
    /// ([`Row::is_problem`]) among
    /// every row before it cuts them, so `crumbkit::finish` exits with the
    /// problem exit and its note counts every such row although the page
    /// holds fewer. A listing from [`Listing::new`] or
    /// [`Listing::uncounted`] holds one page that the tool fetched, so
    /// crumbkit counts only the rows of that page.
    #[must_use]
    pub fn from_all(mut rows: Vec<R>, limit: Option<usize>) -> Self {
        let total = rows.len();
        let problems = match limit {
            // A count of 0 over every row is also the count of the rows the
            // listing keeps, so it is stored as `None`, as for a listing that
            // its limit did not cut.
            Some(limit) if limit < total => {
                Some(crate::row::problems::count(&rows)).filter(|&n| n > 0)
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

    /// Sets the phrase that names one row, such as `book` beside the
    /// description `books`. A listing without a total uses it when it shows
    /// one row, so its footer reads `Showing 1 book, and more match. --all
    /// shows every one.` and its cut line `Showing 1 book, because ...`.
    /// Without it, such a listing names one row with its description, or as
    /// `row` when it has none. A listing with a total never uses it, because
    /// its footer counts against the total, as in `Showing 1 of 6 books.`
    #[must_use]
    pub fn singular(mut self, phrase: impl Into<String>) -> Self {
        self.singular = Some(phrase.into());
        self
    }

    /// Sets the summary: a value that the text output prints with its
    /// `Display` as one line above the table, and that the JSON document
    /// holds under `summary`, serialized with its `Serialize` and described
    /// by its `JsonSchema`. A summary
    /// holds what describes the whole result, such as the figures the rows
    /// were counted from, so a program reads it as fields and a person reads
    /// it as a sentence.
    ///
    /// When the summary's `Display` writes no text, the text output prints
    /// no summary line, so the header is the first line, and the JSON
    /// document still holds the summary. A tool can therefore set a summary
    /// on every run so that its JSON always holds the figures, and write
    /// text only on the runs that have something to say. A `Display` that
    /// writes only spaces still prints its line.
    ///
    /// A listing with a summary line and no rows to print prints the summary line
    /// alone, with no header, in every text view (ADR-0153).
    #[must_use]
    pub fn summary<T: Serialize + fmt::Display + JsonSchema>(self, summary: T) -> Listing<R, T> {
        Listing {
            rows: self.rows,
            total: self.total,
            more: self.more,
            capped: self.capped,
            description: self.description,
            singular: self.singular,
            summary: Some(summary),
            note: self.note,
            long_note: self.long_note,
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

    /// Sets the long note: a line that the text output prints on standard
    /// error after the note and before any next step, under `--long` only,
    /// such as `Sizes are blocks on disk, and a hard-linked file counts
    /// once.` The default text, `--fields` and the JSON document leave it
    /// out, so a long note holds detail that a person asks for with
    /// `--long`, such as how the figures of the rows were measured.
    ///
    /// When the row type has no long columns, the command offers `--long`
    /// only when the listing's document declares the note with
    /// `Document::long_noted` (ADR-0139). Help then promises the note, so
    /// such a command sets it on every listing it returns.
    #[must_use]
    pub fn long_note(mut self, text: impl Into<String>) -> Self {
        self.long_note = Some(text.into());
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

    /// Says whether the tool read only part of its source, such as the
    /// newest 100 records of a store that holds more. With `capped` true,
    /// the listing's total counts the rows that the tool read, and the
    /// source holds more rows beyond them. With `false` it changes nothing,
    /// so a tool passes the answer of its own read.
    ///
    /// A capped listing has no exact total, so its JSON document leaves out
    /// `total` and holds `"truncated": true` (ADR-0087). Its text footer
    /// gives the total as a lower bound, such as `Showing 20 of at least 100
    /// records. --all shows 100 of them.` When the listing shows every row
    /// that the tool read, as with `--all`, its footer reads `Showing 100
    /// records, and more match.`, because `--all` shows nothing more.
    ///
    /// It applies to a listing from [`Listing::new`], [`Listing::from_all`],
    /// `crumbkit::Page::listing` and `crumbkit::Page::first_of`. A listing
    /// from [`Listing::uncounted`] has no total to give as a lower bound,
    /// so on such a listing it changes nothing.
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Record { id: u32 }
    /// # impl crumbkit::Row for Record { const COLUMNS: &'static [&'static str] = &["id"]; }
    /// use crumbkit::Listing;
    ///
    /// // The tool read the newest 100 records, and its store holds older ones.
    /// let read: Vec<Record> = (1..=100).map(|id| Record { id }).collect();
    /// let history_was_cut = true;
    /// let listing = Listing::from_all(read, Some(20))
    ///     .capped(history_was_cut)
    ///     .described("records");
    /// ```
    #[must_use]
    pub const fn capped(mut self, capped: bool) -> Self {
        self.capped = capped && self.total.is_some();
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
        self.total.filter(|_| !self.capped)
    }

    fn at_least(&self) -> Option<usize> {
        self.total.filter(|_| self.capped)
    }

    fn more(&self) -> bool {
        self.capped
            || self
                .total
                .map_or(self.more, |total| self.rows.len() < total)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn singular(&self) -> Option<&str> {
        self.singular.as_deref()
    }

    fn summary(&self) -> Option<&S> {
        self.summary.as_ref()
    }

    fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    fn long_note(&self) -> Option<&str> {
        self.long_note.as_deref()
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
