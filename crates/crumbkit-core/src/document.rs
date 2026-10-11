//! The typed id of a JSON document.

use std::fmt;
use std::marker::PhantomData;

use crate::segment::is_segment;

#[cfg(test)]
mod tests;

/// The id of a JSON document whose `data` holds a `T`, such as
/// `shelf/books-list/v1`.
///
/// A tool declares one constant for each result it prints, and passes it to
/// `crumbkit::finish` with the result. The type parameter ties the id to
/// the result type, so the compiler rejects a document paired with the wrong
/// result.
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit_core::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit_core::Document;
///
/// const BOOKS_LIST: Document<Vec<Book>> = Document::new("shelf", "books-list", 1);
///
/// assert_eq!(BOOKS_LIST.to_string(), "shelf/books-list/v1");
/// ```
pub struct Document<T> {
    /// The component that owns the document, such as `shelf`.
    component: &'static str,
    /// The document's name within its component, such as `books-list`.
    name: &'static str,
    /// The version, which starts at 1.
    version: u32,
    /// `PhantomData<fn() -> T>` ties the document to `T` without holding a
    /// `T`. A function pointer is `Send` and `Sync` whatever it returns, so
    /// `Document<T>` is `Send` and `Sync` for every `T`; `PhantomData<T>`
    /// would make it so only when `T` is. The `Clone` and `Copy` impls below
    /// are written by hand because `#[derive]` would require `T: Copy`, so
    /// `Document<T>` is `Copy` for every `T` as well.
    result: PhantomData<fn() -> T>,
}

impl<T> Document<T> {
    /// Returns the document `component/name/vN`.
    ///
    /// # Panics
    ///
    /// Panics unless `component` is non-empty lowercase ASCII letters, digits
    /// and underscores, `name` is non-empty lowercase ASCII letters, digits
    /// and dashes, and `version` is at least 1. A `const` that calls it with
    /// such arguments fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit_core::Document<()> = crumbkit_core::Document::new("Shelf", "books-list", 1);
    /// ```
    #[must_use]
    pub const fn new(component: &'static str, name: &'static str, version: u32) -> Self {
        assert!(
            is_segment(component, b'_'),
            "a document component is non-empty lowercase ASCII letters, digits and underscores"
        );
        assert!(
            is_segment(name, b'-'),
            "a document name is non-empty lowercase ASCII letters, digits and dashes"
        );
        assert!(version >= 1, "a document version is at least 1");
        Self {
            component,
            name,
            version,
            result: PhantomData,
        }
    }
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
