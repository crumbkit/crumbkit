//! A page of rows with the total number of matching rows, as far as the
//! tool counted them.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};

use crate::{Row, Shape, Step};

#[cfg(test)]
mod tests;

/// One page of rows, the total number of rows that matched when the tool
/// counted them, an optional phrase that names them, an optional summary, an optional note, an
/// optional long note and any next steps.
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
/// `--long` the long note, and then each next step ([`Listing::step`]), such
/// as the line `Next: Show books by id, up to 100 in one call.` followed by
/// the command `  shelf books show <ID>...`. The JSON document holds the
/// steps under `next`.
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Call, Listing, Step};
///
/// let books: Vec<Book> = (1..=6).map(|id| Book { id }).collect();
/// let listing = Listing::from_all(books, Some(4))
///     .described("books")
///     .step(
///         Step::new("Show books by id, up to 100 in one call.")
///             .call(Call::new(&["books", "show"]).arg("ids")),
///     );
/// ```
///
/// The summary `S` is a value that prints as one or more lines above the
/// table in text and as an object under `summary` in JSON. A summary whose
/// `Display` writes no text prints no line. A listing without one has the summary
/// type [`NoSummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing<R: Row, S = NoSummary> {
    /// The rows to print, in order.
    rows: Vec<R>,
    /// The number of rows that matched, the number that the tool read from
    /// a source that holds more, or neither when the tool did not count
    /// them. A count is at least `rows.len()`.
    total: Total,
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
    /// The next steps that the output names, in order.
    steps: Vec<Step>,
    /// The most rows that the command's `--limit` takes, which the footer
    /// names, or `None` when `--limit` has no upper bound.
    bound: Option<usize>,
    /// The number of rows that report a problem ([`Row::is_problem`])
    /// among every row that [`Listing::from_all`] was given, when its limit
    /// cut some of them and at least one of them reports a problem, or
    /// `None` when the listing counts the rows it holds.
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
            total: Total::Exact(total),
            description: None,
            singular: None,
            summary: None,
            note: None,
            long_note: None,
            steps: Vec::new(),
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
        listing.total = Total::Unknown { more };
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
    /// `Display` above the table, and that the JSON document
    /// holds under `summary`, serialized with its `Serialize` and described
    /// by its `JsonSchema`. A summary
    /// holds what describes the whole result, such as the figures the rows
    /// were counted from, so a program reads it as fields and a person reads
    /// it as a sentence.
    ///
    /// Each line feed that the `Display` writes ends a line, so a summary
    /// can print several lines above the table, such as a few counts. A
    /// final line feed adds no empty line. Each line escapes its other
    /// control characters, such as a tab or an escape, and its Unicode
    /// format characters, as a table cell escapes them (ADR-0158). The
    /// output budget counts every summary line.
    ///
    /// When the summary's `Display` writes no text, the text output prints
    /// no summary line, so the header is the first line, and the JSON
    /// document still holds the summary. A tool can therefore set a summary
    /// on every run so that its JSON always holds the figures, and write
    /// text only on the runs that have something to say. A `Display` that
    /// writes only spaces still prints its line.
    ///
    /// A listing with a summary and no rows to print prints the summary
    /// lines alone, with no header, in every text view (ADR-0153).
    #[must_use]
    pub fn summary<T: Serialize + fmt::Display + JsonSchema>(self, summary: T) -> Listing<R, T> {
        Listing {
            rows: self.rows,
            total: self.total,
            description: self.description,
            singular: self.singular,
            summary: Some(summary),
            note: self.note,
            long_note: self.long_note,
            steps: self.steps,
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

    /// Adds `step` after the steps already added. The text output prints each
    /// step on standard error after the footer and the notes, in the order
    /// added: the line `Next: {sentence}`, then each of its commands indented
    /// by two spaces. The JSON document holds each step as an entry of
    /// `next`, as `{"text": {sentence}, "commands": [...]}` (ADR-0155,
    /// ADR-0167, ADR-0207).
    ///
    /// `crumbkit::finish` renders each of the step's calls against the
    /// command that the run's `crumbkit::Run` holds, and leaves the whole
    /// step out of the text and the JSON when one of them does not resolve.
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Book { id: u32 }
    /// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
    /// use crumbkit::{Call, Listing, Program, Step};
    ///
    /// const GIT: Program = Program::new("git");
    ///
    /// let books: Vec<Book> = (1..=6).map(|id| Book { id }).collect();
    /// let listing = Listing::from_all(books, None).step(
    ///     Step::new("Commit the shelf, then list the books.")
    ///         .run(GIT, ["commit", "-m", "Add books"])
    ///         .call(Call::new(&["books", "list"])),
    /// );
    /// ```
    #[must_use]
    pub fn step(mut self, step: Step) -> Self {
        self.steps.push(step);
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
        if let (true, Total::Exact(read)) = (capped, self.total) {
            self.total = Total::AtLeast(read);
        }
        self
    }

    /// Returns the rows that the listing shows, in order. A tool's test
    /// reads the rows of the listing that its handler returned here.
    #[must_use]
    pub fn rows(&self) -> &[R] {
        &self.rows
    }

    /// Returns the listing's total: the number of rows that matched, the
    /// number that the tool read from a source that holds more
    /// ([`Listing::capped`]), or no count ([`Listing::uncounted`]).
    /// [`Total::exact`] returns the number of rows that matched, which the
    /// footer and the JSON document's `total` give, or `None` for the
    /// other two.
    #[must_use]
    pub const fn total(&self) -> Total {
        self.total
    }

    /// Returns the summary that [`Listing::summary`] set, or `None` when
    /// the tool set none. The getter has the `get_` prefix because the
    /// builder takes the plain name, as `Tool` and `Leaf` getters do.
    #[must_use]
    pub const fn get_summary(&self) -> Option<&S> {
        self.summary.as_ref()
    }

    /// Returns the note that [`Listing::note`] set, or `None` when the tool
    /// set none, as [`Batch::get_note`](crate::Batch::get_note) does.
    #[must_use]
    pub fn get_note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// Serializes the listing as the array of its rows. `crumbkit::finish`
/// writes the total beside it, under `total`.
impl<R: Row, M> Serialize for Listing<R, M> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<R: Row, S: Serialize + fmt::Display + JsonSchema> crate::Rows for Listing<R, S> {
    fn rows(&self) -> &[R] {
        &self.rows
    }

    fn total(&self) -> Option<usize> {
        self.total.exact()
    }
}

impl<R: Row, S: Serialize + fmt::Display + JsonSchema> crate::row::Sealed for Listing<R, S> {
    type Row = R;
    type Summary = S;
    const SHAPE: Shape = Shape::Listing;

    fn at_least(&self) -> Option<usize> {
        match self.total {
            Total::AtLeast(read) => Some(read),
            Total::Exact(_) | Total::Unknown { .. } => None,
        }
    }

    fn more(&self) -> bool {
        match self.total {
            Total::Exact(total) => self.rows.len() < total,
            Total::AtLeast(_) => true,
            Total::Unknown { more } => more,
        }
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn singular(&self) -> Option<&str> {
        self.singular.as_deref()
    }

    fn summary(&self) -> Option<&S> {
        self.get_summary()
    }

    fn note(&self) -> Option<&str> {
        self.get_note()
    }

    fn long_note(&self) -> Option<&str> {
        self.long_note.as_deref()
    }

    fn steps(&self) -> &[Step] {
        &self.steps
    }

    fn limit_bound(&self) -> Option<usize> {
        self.bound
    }

    fn problems(&self) -> usize {
        self.problems
            .unwrap_or_else(|| crate::row::problems::count(&self.rows))
    }
}

/// The total of a [`Listing`]: how many rows matched, as far as the tool
/// knows. [`Listing::total`] returns it.
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Listing, Total};
///
/// let books: Vec<Book> = (1..=6).map(|id| Book { id }).collect();
/// let listing = Listing::from_all(books, Some(4));
///
/// assert_eq!(listing.total(), Total::Exact(6));
/// assert_eq!(listing.total().exact(), Some(6));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Total {
    /// The tool counted every matching row, and this many matched.
    /// [`Listing::new`] and [`Listing::from_all`] give it. The listing
    /// leaves rows out when it shows fewer rows than the count. The JSON
    /// document prints the count under `total`.
    Exact(usize),
    /// The tool read this many rows from a source that holds more, so at
    /// least this many matched. [`Listing::capped`] gives it. The listing
    /// always leaves rows out, and its text footer gives the count as a
    /// lower bound.
    AtLeast(usize),
    /// The tool did not count the matching rows. [`Listing::uncounted`]
    /// gives it. `more` says whether the source holds rows after the
    /// listing's rows, and the listing leaves rows out when it does.
    Unknown {
        /// Whether the source holds rows after the listing's rows.
        more: bool,
    },
}

impl Total {
    /// Returns the number of rows that matched for an [`Total::Exact`]
    /// total, or `None` for a lower bound or an unknown total.
    #[must_use]
    pub const fn exact(self) -> Option<usize> {
        match self {
            Self::Exact(total) => Some(total),
            Self::AtLeast(_) | Self::Unknown { .. } => None,
        }
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
