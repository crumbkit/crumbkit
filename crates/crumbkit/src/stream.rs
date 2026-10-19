//! The result type of a command that prints a stream of rows.

use std::marker::PhantomData;

/// The result type of the document that a stream command prints: rows of
/// type `R`, printed one at a time as each arrives, such as a watch or a
/// log follower.
///
/// A tool declares the stream's document as a `Document<Stream<R>>`, and
/// that constant decides the command's form everywhere: its help leaf
/// declares it with `Leaf::stream`, its handler opens the stream with
/// `crumbkit::stream`, and `commands list` and `schemas list` show the form
/// `stream`. With `--json`, each row prints as one JSON document on its own
/// line, which makes the output JSON Lines (ADR-0041).
///
/// No value of the type exists. It only names the row type, so the compiler
/// rejects a stream document passed to `crumbkit::finish` and a document of
/// one result passed to `crumbkit::stream`.
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Entry { position: u32 }
/// # impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &["position"]; }
/// use crumbkit::{Document, Stream};
///
/// const ENTRIES_STREAM: Document<Stream<Entry>> =
///     Document::new("reading", "entries-stream", 1, "Each entry of a reading list.");
///
/// assert_eq!(ENTRIES_STREAM.to_string(), "reading/entries-stream/v1");
/// ```
#[derive(Debug)]
pub struct Stream<R> {
    /// `PhantomData<fn() -> R>` ties the type to `R` without holding an
    /// `R`, as [`Document`](crate::Document) does, so the type is `Send`
    /// and `Sync` for every `R`.
    /// The field is private and crumbkit never builds the type, so no
    /// value of it exists.
    row: PhantomData<fn() -> R>,
}
