//! `shelf books summarize`, which describes a selection of the catalogue in
//! one sentence.

use std::collections::BTreeSet;
use std::fmt;

use crumbkit::{Document, Prose};
use schemars::JsonSchema;
use serde::Serialize;

use super::{BooksError, CATALOGUE, Form, find};
use crate::cli::BooksSummarize;

/// The document that `shelf books summarize` prints: the counts and years of
/// the books for the given ids, and a failure naming the ids the catalogue
/// lacks.
pub(crate) const BOOKS_SUMMARIZE: Document<Prose<Summary, BooksError>> = Document::new(
    "shelf",
    "books-summarize",
    1,
    "The counts and years of the books for the given ids, and a failure for the ids the catalogue lacks.",
);

/// The counts, years and forms of the books found.
// The doc comment above is the description in the document's schema. The
// `Display` impl prints the summary as one sentence, such as `1 book by 1
// author, first published in 1818: 1 novel.`
#[derive(Debug, Default, Serialize, JsonSchema)]
pub(crate) struct Summary {
    /// The number of books found.
    books: u32,
    /// The number of distinct authors of the books found.
    authors: u32,
    /// The year the earliest book was first published, or null when no book
    /// was found.
    earliest: Option<u16>,
    /// The year the latest book was first published, or null when no book
    /// was found.
    latest: Option<u16>,
    /// The number of novels among the books found.
    novels: u32,
    /// The number of collections of short stories among the books found.
    short_story_collections: u32,
}

/// Prints the counts as one sentence that ends with a full stop, and
/// `No books.` when the selection holds no book.
impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (Some(earliest), Some(latest)) = (self.earliest, self.latest) else {
            return f.write_str("No books.");
        };
        write!(
            f,
            "{} by {}, first published ",
            counted(self.books, "book", "books"),
            counted(self.authors, "author", "authors"),
        )?;
        if earliest == latest {
            write!(f, "in {earliest}: ")?;
        } else {
            write!(f, "from {earliest} to {latest}: ")?;
        }
        let mut forms = Vec::new();
        if self.novels > 0 {
            forms.push(counted(self.novels, "novel", "novels"));
        }
        if self.short_story_collections > 0 {
            forms.push(counted(
                self.short_story_collections,
                "collection of short stories",
                "collections of short stories",
            ));
        }
        write!(f, "{}.", forms.join(" and "))
    }
}

/// Returns `count` followed by `one` when it is 1 and by `many` otherwise.
fn counted(count: u32, one: &str, many: &str) -> String {
    let noun = if count == 1 { one } else { many };
    format!("{count} {noun}")
}

/// Returns the summary of the books that `args` names, or of the whole
/// catalogue when it names none. A repeated id counts once.
///
/// The result holds one `BooksError::NotFound` naming every id that the
/// catalogue lacks, each once and in the order given, and no failure when
/// every id names a book.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every shelf handler returns Result<_, BooksError>, so main passes each result to finish in the same way"
)]
pub(crate) fn summarize(args: &BooksSummarize) -> Result<Prose<Summary, BooksError>, BooksError> {
    let mut ids = Vec::new();
    for &id in &args.ids {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if args.ids.is_empty() {
        ids = CATALOGUE.iter().map(|book| book.id).collect();
    }
    let mut summary = Summary::default();
    let mut authors = BTreeSet::new();
    let mut missing = Vec::new();
    for id in ids {
        let Ok(book) = find(id) else {
            missing.push(id);
            continue;
        };
        summary.books += 1;
        authors.insert(book.author);
        summary.earliest = Some(
            summary
                .earliest
                .map_or(book.year, |year| year.min(book.year)),
        );
        summary.latest = Some(summary.latest.map_or(book.year, |year| year.max(book.year)));
        match book.form {
            Form::Novel => summary.novels += 1,
            Form::ShortStories => summary.short_story_collections += 1,
        }
    }
    summary.authors = u32::try_from(authors.len()).unwrap_or(u32::MAX);
    let failures = if missing.is_empty() {
        Vec::new()
    } else {
        vec![BooksError::NotFound { ids: missing }]
    };
    Ok(Prose::with_failures(summary, failures))
}
