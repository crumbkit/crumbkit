//! The books that shelf serves, with shelf's own value type, the number of
//! pages in a book, and the reading time that the catalogue derives from it.

use std::fmt;

use crumbkit::schemars::{JsonSchema, Schema, SchemaGenerator};
use crumbkit::{Duration, Shown};
use serde::Serializer;

use super::{Book, Form};

#[cfg(test)]
mod tests;

/// A number of pages, which reads `432 pages` in text and is the number
/// `432` in JSON, so a program reading `--json` compares it as a number.
///
/// It is shelf's worked example of a value type of its own (ADR-0147): the
/// `Display` is the text form, `Shown::json` and `Shown::schema` give the
/// JSON form, and `crumbkit::shown!` writes `Serialize` and `JsonSchema`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Pages(pub(crate) u32);

impl fmt::Display for Pages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} pages", self.0)
    }
}

impl Shown for Pages {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        u32::json_schema(generator)
    }
}

crumbkit::shown!(Pages);

/// Returns the time it takes to read `pages` at two minutes a page.
pub(super) const fn reading(pages: u32) -> Duration {
    Duration(std::time::Duration::from_secs(pages as u64 * 120))
}

/// The catalogue that shelf serves. Every book in it is in the public domain.
pub(super) const CATALOGUE: [Book; 6] = [
    Book {
        id: 1,
        title: "Pride and Prejudice",
        author: "Jane Austen",
        year: 1813,
        form: Form::Novel,
        pages: Pages(432),
        reading: reading(432),
    },
    Book {
        id: 2,
        title: "Frankenstein",
        author: "Mary Shelley",
        year: 1818,
        form: Form::Novel,
        pages: Pages(280),
        reading: reading(280),
    },
    Book {
        id: 3,
        title: "Moby-Dick",
        author: "Herman Melville",
        year: 1851,
        form: Form::Novel,
        pages: Pages(635),
        reading: reading(635),
    },
    Book {
        id: 4,
        title: "Great Expectations",
        author: "Charles Dickens",
        year: 1861,
        form: Form::Novel,
        pages: Pages(544),
        reading: reading(544),
    },
    Book {
        id: 5,
        title: "Middlemarch",
        author: "George Eliot",
        year: 1871,
        form: Form::Novel,
        pages: Pages(880),
        reading: reading(880),
    },
    Book {
        id: 6,
        title: "The Adventures of Sherlock Holmes",
        author: "Arthur Conan Doyle",
        year: 1892,
        form: Form::ShortStories,
        pages: Pages(307),
        reading: reading(307),
    },
];
