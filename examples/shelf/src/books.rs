//! The built-in book catalogue.

use std::fmt;
use std::num::NonZeroUsize;

use crumbkit::{Batch, Call, Code, Document, Failure, Listing, Recovery, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{BooksList, BooksShow};

#[cfg(test)]
mod tests;

/// The document that `shelf books list` prints: one page of the catalogue and
/// the number of books in it.
pub(crate) const BOOKS_LIST: Document<Listing<Book>> = Document::new("shelf", "books-list", 1);

/// The document that `shelf books show` prints: the books for the given ids
/// that the catalogue holds, and a failure naming the ids it lacks.
pub(crate) const BOOKS_SHOW: Document<Batch<Book, BooksError>> =
    Document::new("shelf", "books-show", 1);

/// The number of threads that `show` looks up books on. The catalogue
/// lookups are in memory, so one worker runs them in the calling thread; a
/// tool that calls a network API raises it.
const WORKERS: NonZeroUsize = NonZeroUsize::MIN;

/// One book in the catalogue. `show` clones a book for each id it is given.
#[derive(Clone, Serialize, JsonSchema)]
pub(crate) struct Book {
    /// The book's id, unique within the catalogue.
    pub(crate) id: u32,
    /// The book's title.
    pub(crate) title: &'static str,
    /// The book's author.
    pub(crate) author: &'static str,
    /// The year the book was first published.
    pub(crate) year: u16,
    /// The book's literary form.
    pub(crate) form: Form,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title", "author", "year"];
    const LONG_COLUMNS: &'static [&'static str] = &["form"];
}

/// The literary form of a book, which serializes in lower case.
#[derive(Clone, Serialize, JsonSchema)]
pub(crate) enum Form {
    /// One long work of prose fiction.
    #[serde(rename = "novel")]
    Novel,
    /// A collection of short stories.
    #[serde(rename = "short stories")]
    ShortStories,
}

/// The catalogue that shelf serves. Every book in it is in the public domain.
const CATALOGUE: [Book; 6] = [
    Book {
        id: 1,
        title: "Pride and Prejudice",
        author: "Jane Austen",
        year: 1813,
        form: Form::Novel,
    },
    Book {
        id: 2,
        title: "Frankenstein",
        author: "Mary Shelley",
        year: 1818,
        form: Form::Novel,
    },
    Book {
        id: 3,
        title: "Moby-Dick",
        author: "Herman Melville",
        year: 1851,
        form: Form::Novel,
    },
    Book {
        id: 4,
        title: "Great Expectations",
        author: "Charles Dickens",
        year: 1861,
        form: Form::Novel,
    },
    Book {
        id: 5,
        title: "Middlemarch",
        author: "George Eliot",
        year: 1871,
        form: Form::Novel,
    },
    Book {
        id: 6,
        title: "The Adventures of Sherlock Holmes",
        author: "Arthur Conan Doyle",
        year: 1892,
        form: Form::ShortStories,
    },
];

/// Returns the books that `args` asks for, in id order, with the number of
/// books in the catalogue as the total.
///
/// The catalogue is a constant in memory, so this builds every book and
/// keeps the first page with `Listing::from_all`.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every shelf handler returns Result<_, BooksError>, so main passes each result to finish in the same way"
)]
pub(crate) fn list(args: &BooksList) -> Result<Listing<Book>, BooksError> {
    Ok(Listing::from_all(Vec::from(CATALOGUE), args.page.limit())
        .described("books")
        .next(show_call(), "takes up to 100 ids in one call."))
}

/// Returns the call that the output of `shelf books list` names as its next
/// step: `shelf books show <ID>...`.
pub(crate) fn show_call() -> Call {
    Call::new(&["books", "show"]).arg("ids")
}

/// Returns a batch of the books that `args` names, in the order given, so a
/// repeated id returns its book again.
///
/// The batch holds one `BooksError::NotFound` naming every id that the
/// catalogue lacks, each once and in the order given, and no failure when
/// every id names a book.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every shelf handler returns Result<_, BooksError>, so main passes each result to finish in the same way"
)]
pub(crate) fn show(args: &BooksShow) -> Result<Batch<Book, BooksError>, BooksError> {
    let mut books = Vec::with_capacity(args.ids.len());
    let mut missing = Vec::new();
    for result in crumbkit::fetch_each(&args.ids, WORKERS, |&id| find(id)) {
        match result {
            Ok(book) => books.push(book),
            Err(id) if !missing.contains(&id) => missing.push(id),
            Err(_) => {}
        }
    }
    let failures = if missing.is_empty() {
        Vec::new()
    } else {
        vec![BooksError::NotFound { ids: missing }]
    };
    Ok(Batch::new(books, failures))
}

/// Returns the book with `id`, or `id` itself as the error when the
/// catalogue has no such book.
fn find(id: u32) -> Result<Book, u32> {
    CATALOGUE
        .iter()
        .find(|book| book.id == id)
        .cloned()
        .ok_or(id)
}

/// The code of a failure that names a book the catalogue lacks.
const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");

/// A failure of a books command.
#[derive(Debug)]
pub(crate) enum BooksError {
    /// The catalogue has no book with these ids. Each id appears once, in
    /// the order the caller gave it.
    NotFound {
        /// The ids that name no book.
        ids: Vec<u32>,
    },
}

impl BooksError {
    /// Every code that a `BooksError` returns. Only shelf's test reads the
    /// list, which it passes to crumbkit-test's codes check.
    #[cfg(test)]
    pub(crate) const CODES: &'static [&'static Code] = &[&NOT_FOUND];
}

/// Prints `the catalogue has no book 7` for one id, and `the catalogue has no
/// books 7, 9` for more.
impl fmt::Display for BooksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { ids } => {
                let noun = if ids.len() == 1 { "book" } else { "books" };
                write!(f, "the catalogue has no {noun} ")?;
                for (index, id) in ids.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{id}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for BooksError {}

/// A missing book's next step is `shelf books list`, which prints the ids
/// that exist.
impl Failure for BooksError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &NOT_FOUND,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NotFound { .. } => Recovery::Run(Call::new(&["books", "list"])),
        }
    }
}
