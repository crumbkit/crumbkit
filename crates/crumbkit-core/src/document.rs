//! The typed id of a JSON document.

use std::fmt;
use std::marker::PhantomData;

use crate::segment::{is_component, is_segment};

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
/// document unchanged while the tool migrates (ADR-0036).
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
    /// Whether `crumbkit::finish` prints the result inside crumbkit's
    /// envelope, with `schema` and `data`, which is true unless the tool
    /// called `without_envelope`.
    envelope: bool,
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
            envelope: true,
            result: PhantomData,
        }
    }

    /// Returns the same document without crumbkit's envelope. With `--json`,
    /// `crumbkit::finish` then prints the result alone, as its type
    /// serializes it with every field, without `schema`, `total` or
    /// `failures`, so a command can print an existing document unchanged and
    /// keep its version while a tool migrates.
    ///
    /// It is an opt-out from crumbkit's conventions. crumbkit-test's
    /// `check_document_ids` fails on a leaf whose document has no envelope
    /// unless the tool opts out of the envelope rule for the command with a
    /// stated reason, and `check_opt_outs` reports what the opt-out costs
    /// (ADR-0036).
    #[must_use]
    pub const fn without_envelope(mut self) -> Self {
        self.envelope = false;
        self
    }

    /// Reports whether `crumbkit::finish` prints the result inside
    /// crumbkit's envelope, which is true unless the tool called
    /// [`Document::without_envelope`].
    #[must_use]
    pub const fn has_envelope(&self) -> bool {
        self.envelope
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

    /// Returns the same id, description and choice of envelope as a
    /// `Document<()>`, which no longer names the result type, so one list
    /// can hold the documents of results of different types. crumbkit's
    /// help leaves keep their documents in this form.
    #[must_use]
    pub const fn untyped(self) -> Document<()> {
        Document {
            component: self.component,
            name: self.name,
            version: self.version,
            description: self.description,
            envelope: self.envelope,
            result: PhantomData,
        }
    }
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
