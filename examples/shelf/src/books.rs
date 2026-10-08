//! The built-in book catalogue and its plain-text rendering.

use std::io::{self, Write};

#[cfg(test)]
mod tests;

/// One book in the catalogue.
pub(crate) struct Book {
    /// The book's id, unique within the catalogue.
    pub(crate) id: u32,
    /// The book's title.
    pub(crate) title: &'static str,
    /// The book's author.
    pub(crate) author: &'static str,
    /// The year the book was first published.
    pub(crate) year: u16,
}

/// The catalogue that shelf serves. Every book in it is in the public domain.
pub(crate) const CATALOGUE: [Book; 6] = [
    Book {
        id: 1,
        title: "Pride and Prejudice",
        author: "Jane Austen",
        year: 1813,
    },
    Book {
        id: 2,
        title: "Frankenstein",
        author: "Mary Shelley",
        year: 1818,
    },
    Book {
        id: 3,
        title: "Moby-Dick",
        author: "Herman Melville",
        year: 1851,
    },
    Book {
        id: 4,
        title: "Great Expectations",
        author: "Charles Dickens",
        year: 1861,
    },
    Book {
        id: 5,
        title: "Middlemarch",
        author: "George Eliot",
        year: 1871,
    },
    Book {
        id: 6,
        title: "The Adventures of Sherlock Holmes",
        author: "Arthur Conan Doyle",
        year: 1892,
    },
];

/// Writes `books` to `out` as a table with a header row and one row per book.
///
/// Each column is as wide as its widest value, and two spaces separate the
/// columns. The last column is not padded, so no line ends in spaces.
pub(crate) fn write_list(books: &[Book], out: &mut impl Write) -> io::Result<()> {
    let id_width = books
        .iter()
        .map(|book| decimal_width(book.id))
        .fold("ID".len(), usize::max);
    let title_width = books
        .iter()
        .map(|book| book.title.chars().count())
        .fold("TITLE".len(), usize::max);
    let author_width = books
        .iter()
        .map(|book| book.author.chars().count())
        .fold("AUTHOR".len(), usize::max);

    writeln!(
        out,
        "{:<id_width$}  {:<title_width$}  {:<author_width$}  YEAR",
        "ID", "TITLE", "AUTHOR"
    )?;
    for book in books {
        writeln!(
            out,
            "{:<id_width$}  {:<title_width$}  {:<author_width$}  {}",
            book.id, book.title, book.author, book.year
        )?;
    }
    Ok(())
}

/// Returns the number of characters that `number` takes when printed in
/// decimal, so the id column can be measured without formatting each id.
fn decimal_width(number: u32) -> usize {
    match number.checked_ilog10() {
        Some(log) => log as usize + 1,
        // Only zero has no logarithm, and it prints as one character.
        None => 1,
    }
}
