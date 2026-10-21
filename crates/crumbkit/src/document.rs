//! The typed id of a JSON document.

use std::fmt;
use std::marker::PhantomData;

use crate::segment::{is_component, is_segment};
use crate::{AsFailure, Batch, Headed, Listing, Row, Rows};

mod envelope;
#[cfg(test)]
mod tests;

pub use envelope::Envelope;

/// The id of a JSON document whose `data` holds a `T`, such as
/// `shelf/books-list/v1`, with a one-line description of the document.
///
/// A tool declares one constant for each result it prints, and passes it to
/// `crumbkit::finish` with the result. The type parameter ties the id to
/// the result type, so the compiler rejects a document paired with the wrong
/// result.
///
/// The component names the document's owner, such as `shelf`, and may be
/// a domain such as `example.dev`. The name of a document that a command
/// prints is the command's path below the tool's name, with its words
/// joined by `-`, so `shelf books list`
/// prints `shelf/books-list/v1` and an agent can tell the id from the
/// command and the command from the id (ADR-0007). A cargo subcommand's
/// path starts below `cargo <name>`, so `cargo crumbkit weigh` prints
/// `crumbkit/weigh/v1`. A document that no command prints on its own, such
/// as `<tool>/failure/v1`, names what it holds. crumbkit-test's
/// `check_document_ids` checks the rule for every leaf of a tool's guide,
/// and a tool can opt out of it for a command with a stated reason.
///
/// The description is what `schemas list` shows for the document and the
/// `description` of its JSON Schema.
///
/// A document has crumbkit's envelope unless the tool chooses another with
/// [`Document::envelope`]. A command can print an existing document
/// unchanged without an envelope while the tool migrates (ADR-0036).
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::Document;
///
/// const BOOKS_LIST: Document<Vec<Book>> =
///     Document::new("shelf", "books-list", 1, "The books in the catalogue.");
///
/// assert_eq!(BOOKS_LIST.to_string(), "shelf/books-list/v1");
/// ```
pub struct Document<T> {
    /// The component that owns the document, such as `shelf` or
    /// `example.dev`.
    component: &'static str,
    /// The document's name within its component, such as `books-list`.
    name: &'static str,
    /// The version, which starts at 1.
    version: u32,
    /// One line that says what the document holds, such as `One page of the
    /// catalogue and the number of books in it.`
    description: &'static str,
    /// The envelope that `crumbkit::finish` prints the result in with
    /// `--json`, which the tool chose with `envelope`.
    envelope: Envelope,
    /// Whether the first answer prints even when it alone passes the output
    /// budget.
    first_always: bool,
    /// The wording of the note that a listing or a stream with no rows
    /// prints, or `None` for crumbkit's default wording.
    empty: Option<&'static str>,
    /// Whether a batch prints each row's bytes in text under a `head`-style
    /// header, which the tool chose with `headed`.
    headed: bool,
    /// The function that returns the bytes of the row at a position of a
    /// headed batch, which `headed` sets. An untyped document has none.
    bytes_of: Option<RowBytes<T>>,
    /// Whether the default text of a listing lists only the rows that
    /// report a problem, which the tool chose with `only_problems`.
    only_problems: bool,
    /// `PhantomData<fn() -> T>` ties the document to `T` without holding a
    /// `T`. A function pointer is `Send` and `Sync` whatever it returns, so
    /// `Document<T>` is `Send` and `Sync` for every `T`; `PhantomData<T>`
    /// would make it so only when `T` is. The `Clone` and `Copy` impls below
    /// are written by hand because `#[derive]` would require `T: Copy`, so
    /// `Document<T>` is `Copy` for every `T` as well.
    result: PhantomData<fn() -> T>,
}

impl<T> Document<T> {
    /// Returns the document `component/name/vN`, which `description`
    /// describes in one line.
    ///
    /// # Panics
    ///
    /// Panics unless `component` is labels joined by single dots, such as
    /// `shelf` or `example.dev`, each label non-empty lowercase ASCII
    /// letters, digits, underscores and dashes, `name` is non-empty lowercase ASCII letters, digits
    /// and dashes, `version` is at least 1, and `description` is non-empty
    /// and holds no line break. A `const` that calls it with such arguments
    /// fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Document<()> =
    ///     crumbkit::Document::new("Shelf", "books-list", 1, "The books.");
    /// ```
    #[must_use]
    pub const fn new(
        component: &'static str,
        name: &'static str,
        version: u32,
        description: &'static str,
    ) -> Self {
        assert!(
            is_component(component),
            "a document component is labels joined by single dots, each non-empty lowercase ASCII letters, digits, underscores and dashes"
        );
        assert!(
            is_segment(name, b'-'),
            "a document name is non-empty lowercase ASCII letters, digits and dashes"
        );
        assert!(version >= 1, "a document version is at least 1");
        assert!(
            is_one_line(description),
            "a document description is one non-empty line"
        );
        Self {
            component,
            name,
            version,
            description,
            envelope: Envelope::Crumbkit,
            first_always: false,
            empty: None,
            headed: false,
            bytes_of: None,
            only_problems: false,
            result: PhantomData,
        }
    }

    /// Returns the same document with its first answer always printed: a
    /// single row or a prose result prints whole even when it passes the
    /// tool's output budget, a listing or a list prints at least its first
    /// row, and a batch prints its first answer. Without it,
    /// `crumbkit::finish` refuses a single result that passes the budget
    /// (ADR-0046), and refuses a batch's first answer that passes the
    /// budget alone with that answer's own failure, in text and with
    /// `--json` (ADR-0084).
    #[must_use]
    pub const fn always_print_first(mut self) -> Self {
        self.first_always = true;
        self
    }

    /// Reports whether [`Document::always_print_first`] was called. It is
    /// hidden because only crumbkit and crumbkit-test read it.
    #[doc(hidden)]
    #[must_use]
    pub const fn always_prints_first(&self) -> bool {
        self.first_always
    }

    /// Returns the same document with `text` as the wording of the note that
    /// a listing with no matching rows, or a stream that ends with no rows,
    /// prints after its header, such as `note: the shelf holds no books`.
    /// crumbkit adds the `note: ` prefix. Without it, the note reads `no
    /// {description} match`, with the listing's description, such as `no
    /// books match` (ADR-0050).
    ///
    /// # Panics
    ///
    /// Panics unless `text` is non-empty and holds no line break. A `const`
    /// that calls it with such a text fails to compile.
    #[must_use]
    pub const fn when_empty(mut self, text: &'static str) -> Self {
        assert!(is_one_line(text), "an empty note is one non-empty line");
        self.empty = Some(text);
        self
    }

    /// Returns the wording that [`Document::when_empty`] set, or `None` when
    /// the document uses crumbkit's default wording.
    #[must_use]
    pub(crate) const fn empty_note(&self) -> Option<&'static str> {
        self.empty
    }

    /// Returns the component that owns the document, such as `shelf`.
    #[must_use]
    pub const fn component(&self) -> &'static str {
        self.component
    }

    /// Returns the document with `component` in place of its own, which
    /// [`ToolName`](crate::ToolName) checked is a document component.
    pub(crate) const fn with_component(self, component: &'static str) -> Self {
        Self { component, ..self }
    }

    /// Returns the document's name within its component, such as
    /// `books-list`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the document's version, such as 1.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the one-line description of the document.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        self.description
    }

    /// Reports whether [`Document::headed`] was called, so a batch prints
    /// each row's bytes in text under a `head`-style header. It is hidden
    /// because only crumbkit and crumbkit-test read it.
    #[doc(hidden)]
    #[must_use]
    pub const fn is_headed(&self) -> bool {
        self.headed
    }

    /// Returns the function that [`Document::headed`] set, which returns
    /// the bytes of the row at a position among a batch's rows, or `None`
    /// for that row's place when the row holds no bytes or the position is
    /// past the last row. It is `None` for a document that is not headed,
    /// and for every untyped document. `crumbkit::finish` calls it to print
    /// a headed batch.
    #[must_use]
    pub(crate) const fn headed_bytes(&self) -> Option<RowBytes<T>> {
        self.bytes_of
    }

    /// Reports whether [`Document::only_problems`] was called, so the
    /// default text of the listing lists only the rows that report a
    /// problem.
    #[must_use]
    pub(crate) const fn lists_only_problems(&self) -> bool {
        self.only_problems
    }

    /// Returns the same id, description, choice of envelope, budget
    /// settings, empty note, text form and choice of rows as a
    /// `Document<()>`, which no
    /// longer names the result type, so one list can hold the documents of
    /// results of different types. crumbkit's
    /// help leaves keep their documents in this form.
    #[must_use]
    pub const fn untyped(self) -> Document<()> {
        Document {
            component: self.component,
            name: self.name,
            version: self.version,
            description: self.description,
            envelope: self.envelope,
            first_always: self.first_always,
            empty: self.empty,
            headed: self.headed,
            bytes_of: None,
            only_problems: self.only_problems,
            result: PhantomData,
        }
    }
}

impl<R: Headed, E: AsFailure> Document<Batch<R, E>> {
    /// Returns the same batch document with each row's bytes printed in text
    /// the way `head` prints several files, in place of a table.
    ///
    /// When the caller gave more than one key, `crumbkit::finish` prints a
    /// line `==> KEY <==` before each row that prints, with a blank line
    /// before every such line but the first, and then the row's bytes
    /// unchanged with no line feed added. With one key it prints the bytes
    /// alone. A row whose [`Headed::bytes`] returns `None` prints as one
    /// line of compact JSON with every field. A key whose lookup failed gets
    /// no header, and its failure prints on standard error.
    ///
    /// Each row prints whole or not at all. The first row prints past the
    /// output budget when the document is marked with
    /// [`Document::always_print_first`]. Otherwise a first row that passes
    /// the budget alone is refused with its own failure, as the first row
    /// of every batch is, and every later row and every unfetched key are
    /// left out. From the first later row that does not fit, that row,
    /// every later row and every unfetched key are left out. One
    /// over-budget failure names the keys left out.
    ///
    /// With `--json` the batch prints as its ordinary document, with every
    /// field of each row unless `--fields` names some. The leaf offers
    /// neither `--long` nor `--full`, and `parse` refuses `--fields` without
    /// `--json`, because the text has no columns (ADR-0062).
    #[must_use]
    pub const fn headed(mut self) -> Self {
        self.headed = true;
        self.bytes_of = Some(row_bytes::<R, E>);
        self
    }
}

impl<R: Row, S> Document<Listing<R, S>> {
    /// Returns the same listing document with a default text that lists
    /// only the rows that report a problem, as [`Row::is_problem`] decides,
    /// such as the failed checks of a check command (ADR-0138).
    ///
    /// Without `--json`, `--long` or `--fields`, `crumbkit::finish` prints
    /// the summary line, and then the table of the rows that report a
    /// problem, each with its child lines. When no row reports one, the text
    /// is the summary line alone, with no header. The footer, the empty note
    /// of a listing without rows, the note that counts the rows that report
    /// a problem and the exit do not change, and no line counts the rows
    /// that the text leaves out. `--long` and `--fields` print every row,
    /// and `--json` holds every row unchanged.
    ///
    /// The leaf offers `--long` even when the row type has no long columns,
    /// because `--long` lists every row, and its help prints a `Rows:` line
    /// that says so. A row type that sets [`Row::WHOLE`] and names no
    /// columns offers no `--long`, so its text lists every row only through
    /// `--json`.
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Check { package: String, passed: bool }
    /// # impl crumbkit::Row for Check {
    /// #     const COLUMNS: &'static [&'static str] = &["package", "passed"];
    /// #     fn is_problem(&self) -> bool { !self.passed }
    /// # }
    /// use crumbkit::{Document, Listing};
    ///
    /// const CHECKS: Document<Listing<Check>> =
    ///     Document::new("lint", "checks", 1, "The checks of each package.").only_problems();
    /// ```
    #[must_use]
    pub const fn only_problems(mut self) -> Self {
        self.only_problems = true;
        self
    }
}

/// A function that returns the bytes of the row at a position among the
/// rows of a result, or `None` when that row holds none.
type RowBytes<T> = fn(&T, usize) -> Option<&[u8]>;

/// Returns the bytes of the row at `index` among the rows of `batch`, or
/// `None` when that row holds none or `index` is past the last row.
fn row_bytes<R: Headed, E: AsFailure>(batch: &Batch<R, E>, index: usize) -> Option<&[u8]> {
    let row = batch.rows().get(index)?;
    row.bytes().map(crate::Bytes::as_slice)
}

/// Returns whether `text` is non-empty and holds no `\n` or `\r`.
const fn is_one_line(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // A `const fn` cannot call iterator methods, so this loop walks the bytes
    // by index.
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' || bytes[index] == b'\r' {
            return false;
        }
        index += 1;
    }
    true
}

impl<T> Clone for Document<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Document<T> {}

/// Prints the id, such as `shelf/books-list/v1`.
impl<T> fmt::Display for Document<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/v{}", self.component, self.name, self.version)
    }
}

/// Prints the id inside `Document(...)`, such as `Document(shelf/books-list/v1)`.
impl<T> fmt::Debug for Document<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Document({self})")
    }
}
