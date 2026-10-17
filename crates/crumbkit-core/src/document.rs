//! The typed id of a JSON document.

use std::fmt;
use std::marker::PhantomData;

use crate::segment::{is_component, is_segment};
use crate::{AsFailure, Batch, Headed, Row, Rows};

mod keys;
#[cfg(test)]
mod tests;

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
/// as `crumbkit/failure/v1`, names what it holds. crumbkit-test's
/// `check_document_ids` checks the rule for every leaf of a tool's guide,
/// and a tool can opt out of it for a command with a stated reason.
///
/// The description is what `schemas list` shows for the document and the
/// `description` of its JSON Schema.
///
/// A document has crumbkit's envelope unless the tool calls
/// [`Document::without_envelope`], which lets a command print an existing
/// document unchanged while the tool migrates (ADR-0036). A tool whose
/// readers know another envelope declares its keys with
/// [`Document::with_keys`] and keeps crumbkit's paging fields (ADR-0059).
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit_core::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit_core::Document;
///
/// const BOOKS_LIST: Document<Vec<Book>> =
///     Document::new("shelf", "books-list", 1, "The books in the catalogue.");
///
/// assert_eq!(BOOKS_LIST.to_string(), "shelf/books-list/v1");
/// ```
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one independent choice that its own const builder sets, and no two of them form a state machine"
)]
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
    /// `--json`: crumbkit's, one with the keys from `with_keys`, or none
    /// after `without_envelope`.
    envelope: Envelope,
    /// Whether the first answer prints even when it alone passes the output
    /// budget.
    first_always: bool,
    /// The wording of the note that a listing or a stream with no rows
    /// prints, or `None` for crumbkit's default wording.
    empty: Option<&'static str>,
    /// Whether a batch without the envelope prints its failures among its
    /// rows in input order with `--json`, which the tool chose with
    /// `failures_in_order`.
    in_order: bool,
    /// Whether a batch prints each row's bytes in text under a `head`-style
    /// header, which the tool chose with `headed`.
    headed: bool,
    /// Whether a batch answers each key that the output budget leaves out
    /// with a failure of its own, which the tool chose with
    /// `left_out_per_key`.
    per_key: bool,
    /// The name of the failure field under which each over-budget failure
    /// of an answer left out holds its key, which the tool set with
    /// `key_field`.
    key_field: Option<&'static str>,
    /// The function that returns the bytes of the row at a position of a
    /// headed batch, which `headed` sets. An untyped document has none.
    bytes_of: Option<RowBytes<T>>,
    /// The value that the type key holds in place of the id, which the tool
    /// set with `with_type`.
    type_value: Option<&'static str>,
    /// Whether a listing's summary prints its fields at the top of the
    /// document in place of under `summary`, which the tool chose with
    /// `summary_at_top`.
    summary_top: bool,
    /// Whether a listing's JSON leaves out `total`, which the tool chose
    /// with `without_total`.
    no_total: bool,
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
    /// const BAD: crumbkit_core::Document<()> =
    ///     crumbkit_core::Document::new("Shelf", "books-list", 1, "The books.");
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
            in_order: false,
            headed: false,
            per_key: false,
            key_field: None,
            bytes_of: None,
            type_value: None,
            summary_top: false,
            no_total: false,
            result: PhantomData,
        }
    }

    /// Returns the same document without crumbkit's envelope. With `--json`,
    /// `crumbkit::finish` then prints the result alone, as its type
    /// serializes it with every field, without `schema`, `total` or
    /// `failures`, so a command can print an existing document unchanged and
    /// keep its version while a tool migrates. `--full` is refused for such
    /// a document, because `--json` already prints every field. A batch
    /// without the envelope prints its rows alone and its failures as text
    /// on standard error, unless the tool calls `failures_in_order`.
    ///
    /// It is an opt-out from crumbkit's conventions. crumbkit-test's
    /// `check_document_ids` fails on a leaf whose document has no envelope
    /// unless the tool opts out of the envelope rule for the command with a
    /// stated reason, and `check_opt_outs` reports what the opt-out costs
    /// (ADR-0036).
    ///
    /// # Panics
    ///
    /// Panics when the document declares its own keys with
    /// [`Document::with_keys`], because a document has one envelope or
    /// none. A `const` that calls it on such a document fails to compile.
    #[must_use]
    pub const fn without_envelope(mut self) -> Self {
        assert!(
            !matches!(self.envelope, Envelope::Keys { .. }),
            "a document with keys from with_keys keeps its envelope; call with_keys or without_envelope, not both"
        );
        self.envelope = Envelope::Without;
        self
    }

    /// Returns the same document with its first answer always printed: a
    /// single row or a prose result prints whole even when it passes the
    /// tool's output budget, and a listing or a list prints at least its
    /// first row. Without it, `crumbkit::finish` refuses a single result
    /// that passes the budget (ADR-0046).
    #[must_use]
    pub const fn always_print_first(mut self) -> Self {
        self.first_always = true;
        self
    }

    /// Reports whether [`Document::always_print_first`] was called.
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
    pub const fn empty_note(&self) -> Option<&'static str> {
        self.empty
    }

    /// Reports whether [`Document::failures_in_order`] was called, so a
    /// batch without the envelope prints its failures among its rows with
    /// `--json`.
    #[must_use]
    pub const fn prints_failures_in_order(&self) -> bool {
        self.in_order
    }

    /// Reports whether `crumbkit::finish` prints the result inside
    /// crumbkit's envelope, which is true unless the tool called
    /// [`Document::without_envelope`].
    #[must_use]
    pub const fn has_envelope(&self) -> bool {
        !matches!(self.envelope, Envelope::Without)
    }

    /// Returns the component that owns the document, such as `shelf`.
    #[must_use]
    pub const fn component(&self) -> &'static str {
        self.component
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
    /// each row's bytes in text under a `head`-style header.
    #[must_use]
    pub const fn is_headed(&self) -> bool {
        self.headed
    }

    /// Reports whether [`Document::left_out_per_key`] was called, so a batch
    /// answers each key that the output budget leaves out with a failure of
    /// its own.
    #[must_use]
    pub const fn answers_left_out_per_key(&self) -> bool {
        self.per_key
    }

    /// Returns the name that [`Document::key_field`] set, under which each
    /// over-budget failure of an answer left out holds its key, or `None`
    /// when the tool named none.
    #[must_use]
    pub const fn key_field_name(&self) -> Option<&'static str> {
        self.key_field
    }

    /// Returns the function that [`Document::headed`] set, which returns
    /// the bytes of the row at a position among a batch's rows, or `None`
    /// for that row's place when the row holds no bytes or the position is
    /// past the last row. It is `None` for a document that is not headed,
    /// and for every untyped document. `crumbkit::finish` calls it to print
    /// a headed batch.
    #[must_use]
    pub const fn headed_bytes(&self) -> Option<RowBytes<T>> {
        self.bytes_of
    }

    /// Returns the same id, description, choice of envelope and its keys,
    /// budget settings, empty note, text form and answer of each key left out as a `Document<()>`, which no longer names the result type,
    /// so one list can hold the documents of results of different types. crumbkit's
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
            in_order: self.in_order,
            headed: self.headed,
            per_key: self.per_key,
            key_field: self.key_field,
            bytes_of: None,
            type_value: self.type_value,
            summary_top: self.summary_top,
            no_total: self.no_total,
            result: PhantomData,
        }
    }
}

impl<R: Row, E: AsFailure> Document<Batch<R, E>> {
    /// Returns the same batch document with its failures printed in input
    /// order. With `--json`, `crumbkit::finish` then prints one array that
    /// holds each row and each failure document in the order of the keys,
    /// and the failure of the answers that the output budget left out last.
    /// Each failure document holds `key` and `index`, the failure's key and
    /// its position among the keys from 0, so a reader tells a failure from
    /// a row by its `schema` and matches it to its key. Without it, the
    /// array holds the rows alone and the failures print as text on
    /// standard error.
    ///
    /// A document with keys from [`Document::with_keys`] holds that array
    /// under its data key and has no `failures` field, and its rows follow
    /// the view flags as in any document with an envelope (ADR-0059).
    ///
    /// It is a declaration that crumbkit-test's `check_opt_outs` lists under
    /// the customisations, and crumbkit-test's `check_guide` fails on a row
    /// type with a field named `schema`, because such a row could look like
    /// a failure (ADR-0016, ADR-0056).
    ///
    /// # Panics
    ///
    /// Panics when the document has crumbkit's own envelope, which holds its
    /// failures under `failures`. A `const` that calls it on such a document
    /// fails to compile.
    #[must_use]
    pub const fn failures_in_order(mut self) -> Self {
        assert!(
            !matches!(self.envelope, Envelope::Crumbkit),
            "failures in input order need a document without crumbkit's own envelope; call without_envelope or with_keys first"
        );
        self.in_order = true;
        self
    }

    /// Returns the same batch document with each answer that the output
    /// budget leaves out answered by a failure of its own, in place of one
    /// failure that names every key left out.
    ///
    /// Each row cut short and each key that the handler never fetched gets
    /// the over-budget failure under its own key and index, at the key's
    /// position among the batch's other failures. Its message is `the answer
    /// did not fit in the output budget of N bytes`, its only field is
    /// `budget`, and its recovery runs the command with that key alone. A
    /// reader that reads each answer by its key then finds every key
    /// answered once. The first failure in key order decides the exit.
    ///
    /// Each failure costs about 200 bytes of the budget where a key in the
    /// one failure's `left_out` costs a few, so `crumbkit::finish` prints the
    /// most rows that fit beside them, and always the first row. Every answer
    /// left out keeps its own failure, so the JSON document can pass the
    /// budget by the size of those failures: a batch of 10,000 keys whose
    /// first row fills the budget prints about 2 MB of failures after it.
    ///
    /// It is a declaration that crumbkit-test's `check_opt_outs` lists under
    /// the customisations (ADR-0068, ADR-0070).
    #[must_use]
    pub const fn left_out_per_key(mut self) -> Self {
        self.per_key = true;
        self
    }

    /// Returns the same batch document with each over-budget failure of an
    /// answer left out holding its key as text under the failure field
    /// `name`, beside `budget`, so a reader that reads a failure's fields
    /// finds the key where it reads the key of the tool's other failures.
    ///
    /// crumbkit-test's `check_guide` fails a name that breaks the field
    /// name rule unless the guide's `field_names` lists it, and fails the
    /// name `budget`. `check_opt_outs` names the field in the
    /// `left-out-per-key` customisation (ADR-0070).
    ///
    /// # Panics
    ///
    /// Panics unless [`Document::left_out_per_key`] was called first,
    /// because only that declaration gives each answer left out a failure
    /// of its own. A `const` that calls it on such a document fails to
    /// compile.
    #[must_use]
    pub const fn key_field(mut self, name: &'static str) -> Self {
        assert!(
            self.per_key,
            "a key field needs left_out_per_key, which gives each answer left out its own failure"
        );
        self.key_field = Some(name);
        self
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
    /// Each row prints whole or not at all. A row larger than the output
    /// budget on its own fails for its key with the over-budget failure,
    /// unless it is the first row of a document that always prints its first
    /// answer.
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

/// A function that returns the bytes of the row at a position among the
/// rows of a result, or `None` when that row holds none.
type RowBytes<T> = fn(&T, usize) -> Option<&[u8]>;

/// Returns the bytes of the row at `index` among the rows of `batch`, or
/// `None` when that row holds none or `index` is past the last row.
fn row_bytes<R: Headed, E: AsFailure>(batch: &Batch<R, E>, index: usize) -> Option<&[u8]> {
    let row = batch.rows().get(index)?;
    row.bytes().map(crate::Bytes::as_slice)
}

/// The envelope that a document prints its result in with `--json`.
#[derive(Clone, Copy)]
enum Envelope {
    /// crumbkit's envelope, with the id under `schema` and the result under
    /// `data`.
    Crumbkit,
    /// An envelope with the keys that the tool declared with
    /// [`Document::with_keys`] (ADR-0059).
    Keys {
        /// The key that holds the document's type.
        type_key: &'static str,
        /// The key that holds the result.
        data_key: &'static str,
    },
    /// No envelope, after [`Document::without_envelope`].
    Without,
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
