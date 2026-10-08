//! The built-in book catalogue.

use crumbkit::{Call, Document, Listing, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{BooksList, BooksShow};

#[cfg(test)]
mod tests;

/// The document that `shelf books list` prints: one page of the catalogue and
/// the number of books in it.
pub(crate) const BOOKS_LIST: Document<Listing<Book>> = Document::new("shelf", "books-list", 1);

/// The document that `shelf books show` prints: the books for the given ids.
pub(crate) const BOOKS_SHOW: Document<Vec<Book>> = Document::new("shelf", "books-show", 1);

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
pub(crate) fn list(args: &BooksList) -> Listing<Book> {
    Listing::from_all(Vec::from(CATALOGUE), args.page.limit())
        .described("books")
        .next(show_call(), "takes up to 100 ids in one call.")
}

/// Returns the call that the output of `shelf books list` names as its next
/// step: `shelf books show <ID>...`.
pub(crate) fn show_call() -> Call {
    Call::new(&["books", "show"]).arg("ids")
}

/// Returns the books that `args` names, in the order given, so a repeated id
/// returns its book again.
///
/// clap accepts only the ids in the catalogue (see `BooksShow`), so every id
/// finds its book and `filter_map` skips none.
pub(crate) fn show(args: &BooksShow) -> Vec<Book> {
    args.ids
        .iter()
        .filter_map(|&id| CATALOGUE.iter().find(|book| book.id == id).cloned())
        .collect()
}
