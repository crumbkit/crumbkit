//! The built-in book catalogue.

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::num::NonZeroUsize;
use std::path::Path;

use crumbkit::pick::Choice;
use crumbkit::{Batch, Call, Code, Document, Failure, FieldValue, Listing, Recovery, Row};
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode, SourceSpan};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{BooksList, BooksShow};

mod remove;
mod summary;
#[cfg(test)]
mod tests;

use remove::read_removed;
pub(crate) use remove::{BOOKS_REMOVE, remove, removed_file};
pub(crate) use summary::{BOOKS_SUMMARIZE, summarize};

/// The document that `shelf books list` prints: one page of the catalogue and
/// the number of books in it.
pub(crate) const BOOKS_LIST: Document<Listing<Book>> = Document::new(
    "shelf",
    "books-list",
    1,
    "One page of the catalogue and the number of books in it.",
);

/// The document that `shelf books show` prints: the books for the given ids
/// that the catalogue holds, and a failure for each id it lacks.
pub(crate) const BOOKS_SHOW: Document<Batch<Book, BooksError>> = Document::new(
    "shelf",
    "books-show",
    1,
    "The books for the given ids that the catalogue holds, and a failure for each id it lacks.",
);

/// The most ids that `shelf books show` takes in one call, on the command
/// line or in the file that `--from` names.
pub(crate) const MOST_IDS: usize = 100;

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
/// books on the shelf as the total. The shelf is the catalogue without the
/// books that the file at `removed` names, as [`read_removed`] reads it.
///
/// The catalogue is a constant in memory, so this builds every book and
/// keeps the first page with `Page::first_of`, which also gives the listing
/// the bound of `--limit` for its footer.
///
/// # Errors
///
/// Returns `BooksError::RemovedFile` when the file of removed books cannot
/// be read.
pub(crate) fn list(args: &BooksList, removed: Option<&Path>) -> Result<Listing<Book>, BooksError> {
    let removed = read_removed(removed)?;
    let books = CATALOGUE
        .into_iter()
        .filter(|book| !removed.contains(&book.id))
        .collect();
    Ok(args
        .page
        .first_of(books)
        .described("books")
        .next(show_call(), "takes up to 100 ids in one call."))
}

/// Returns the books on the shelf as the choices that a picker offers for
/// the ids of `shelf books show`, as [`choices_from`] reads them from the
/// file that [`removed_file`] names.
///
/// # Errors
///
/// Returns `BooksError::RemovedFile` when the file of removed books cannot
/// be read.
pub(crate) fn choices() -> Result<Vec<Choice>, crumbkit::Error> {
    Ok(choices_from(removed_file().as_deref())?)
}

/// Returns the books on the shelf as choices: the catalogue without the
/// books that the file at `removed` names. Each choice's value is the
/// book's id, and its label is the book's title, author and year.
///
/// # Errors
///
/// Returns `BooksError::RemovedFile` when the file of removed books cannot
/// be read.
pub(crate) fn choices_from(removed: Option<&Path>) -> Result<Vec<Choice>, BooksError> {
    let removed = read_removed(removed)?;
    let books: Vec<Book> = CATALOGUE
        .into_iter()
        .filter(|book| !removed.contains(&book.id))
        .collect();
    Ok(crumbkit::pick::from_rows(&books, "id"))
}

/// Returns the call that the output of `shelf books list` names as its next
/// step: `shelf books show <ID>...`.
pub(crate) fn show_call() -> Call {
    Call::new(&["books", "show"]).arg("ids")
}

/// Returns a batch of the books that `args` names, in the order given, so a
/// repeated id returns its book again. The ids come from the command line,
/// or from the file that `--from` names, as [`read_ids`] reads it.
///
/// The batch holds one `BooksError::NotFound` for each id that the
/// catalogue lacks, in the order given. `show` looks up one chunk of
/// [`WORKERS`] ids at a time, and stops once the books it found take
/// `budget` bytes, the bytes that the run has left, so crumbkit names the
/// ids it did not look up in the over-budget failure.
///
/// # Errors
///
/// Returns `BooksError::UnreadableIdsFile` or `BooksError::BadIdsFile` when
/// the ids file cannot be read or holds a line that is not a book id, and
/// `BooksError::RemovedFile` when the file of removed books cannot be read.
pub(crate) fn show(
    args: &BooksShow,
    removed: Option<&Path>,
    budget: usize,
) -> Result<Batch<Book, BooksError>, BooksError> {
    let ids = match &args.from {
        Some(path) => read_ids(path)?,
        None => args.ids.clone(),
    };
    let removed = read_removed(removed)?;
    let mut batch = Batch::new(&["books", "show"], "ids", &ids);
    for chunk in ids.chunks(WORKERS.get()) {
        if !batch.has_room(budget) {
            break;
        }
        batch.extend(crumbkit::fetch_each(chunk, WORKERS, |&id| {
            find(id, &removed).map_err(|id| BooksError::NotFound { ids: vec![id] })
        }));
    }
    Ok(batch)
}

/// Returns the ids in the file at `path`, in file order. Each line holds one
/// id, with any spaces around it, and a line that holds nothing else is
/// skipped. The file holds at least 1 and at most [`MOST_IDS`] ids, as the
/// command line does, and reading stops one line after the id after the
/// last one allowed.
///
/// # Errors
///
/// Returns `BooksError::UnreadableIdsFile` when the file cannot be read as
/// UTF-8 text, `BooksError::BadIdsFile` at the first line that is not a book
/// id, `BooksError::TooManyIds` at the line of the id after the last one
/// allowed, and `BooksError::EmptyIdsFile` when the file holds no id.
fn read_ids(path: &Path) -> Result<Vec<u32>, BooksError> {
    let name = path.display().to_string();
    let mut text = String::new();
    let problem = match File::open(path) {
        Ok(file) => match parse_ids(&mut BufReader::new(file), &mut text) {
            Ok(ids) => return Ok(ids),
            Err(problem) => problem,
        },
        Err(error) => IdsFileProblem::Unreadable(error),
    };
    Err(match problem {
        IdsFileProblem::Unreadable(error) => BooksError::UnreadableIdsFile { name, error },
        IdsFileProblem::BadLine { span, reason } => BooksError::BadIdsFile {
            file: NamedSource::new(name, text),
            span,
            problem: reason,
        },
        IdsFileProblem::TooMany { span } => BooksError::TooManyIds {
            file: NamedSource::new(name, text),
            span,
        },
        IdsFileProblem::Empty => BooksError::EmptyIdsFile { name },
    })
}

/// Why an ids file gives no ids. [`read_ids`] turns each into a
/// `BooksError` that holds the file's name and, for a line, the text read.
#[derive(Debug)]
enum IdsFileProblem {
    /// The file cannot be read as UTF-8 text.
    Unreadable(io::Error),
    /// The text at `span` is not a book id, for the parser's `reason`.
    BadLine {
        /// The span of the line's text without the spaces around it.
        span: SourceSpan,
        /// Why the text is not a book id.
        reason: String,
    },
    /// The id at `span` comes after the last one allowed.
    TooMany {
        /// The span of the id without the spaces around it.
        span: SourceSpan,
    },
    /// The file holds no id.
    Empty,
}

/// Returns the ids that `reader` holds, one per line, as [`read_ids`]
/// describes. It appends each line it reads to `text`, so `text` holds the
/// file up to the line after the line that a problem points at, and the
/// failure's excerpt shows one line on each side of the problem.
fn parse_ids(reader: &mut impl BufRead, text: &mut String) -> Result<Vec<u32>, IdsFileProblem> {
    let mut ids = Vec::new();
    loop {
        let line_start = text.len();
        let read = reader.read_line(text).map_err(IdsFileProblem::Unreadable)?;
        if read == 0 {
            break;
        }
        let line = &text[line_start..];
        let id = line.trim();
        if id.is_empty() {
            continue;
        }
        let start = line_start + (line.len() - line.trim_start().len());
        let span = SourceSpan::new(start.into(), id.len());
        let problem = if ids.len() == MOST_IDS {
            IdsFileProblem::TooMany { span }
        } else {
            match id.parse::<u32>() {
                Ok(id) => {
                    ids.push(id);
                    continue;
                }
                Err(error) => IdsFileProblem::BadLine {
                    span,
                    reason: error.to_string(),
                },
            }
        };
        // The line after the problem is only context for the excerpt, so a
        // line that cannot be read is left out and the problem stays the
        // one reported. `read_line` leaves `text` unchanged when the line is
        // not UTF-8.
        let _ = reader.read_line(text);
        return Err(problem);
    }
    if ids.is_empty() {
        return Err(IdsFileProblem::Empty);
    }
    Ok(ids)
}

/// Returns the book with `id`, or `id` itself as the error when the
/// catalogue has no such book or `removed` holds `id`.
fn find(id: u32, removed: &[u32]) -> Result<Book, u32> {
    CATALOGUE
        .iter()
        .find(|book| book.id == id && !removed.contains(&id))
        .cloned()
        .ok_or(id)
}

/// The code of a failure that names a book the catalogue lacks.
pub const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");

/// The code of a failure whose ids file cannot be read.
pub const UNREADABLE_IDS_FILE: Code = Code::new("shelf", "books", "unreadable_ids_file");

/// The code of a failure whose ids file holds a line that is not a book id.
pub const BAD_IDS_FILE: Code = Code::new("shelf", "books", "bad_ids_file");

/// The code of a failure whose ids file holds more than 100 ids, the most
/// that `books show` takes.
pub const TOO_MANY_IDS: Code = Code::new("shelf", "books", "too_many_ids");

/// The code of a failure whose ids file holds no id.
pub const EMPTY_IDS_FILE: Code = Code::new("shelf", "books", "empty_ids_file");

/// The code of a failure whose file of removed books cannot be read or
/// written.
pub const REMOVED_FILE: Code = Code::new("shelf", "books", "removed_file");

/// The code of a failure that finds no state directory for the file of
/// removed books.
pub const NO_STATE_DIRECTORY: Code = Code::new("shelf", "books", "no_state_directory");

/// A failure of a books command. A missing book's next step is `shelf books
/// list`, which prints the ids that exist. An ids file failure's next step is
/// advice about the file, and its alternative passes the ids on the command
/// line instead, as `shelf books show <ID>...`.
///
/// `#[derive(Failure)]` also writes `BooksError::CODES`, which shelf's tests
/// pass to crumbkit-test's codes and recovery checks.
#[derive(Debug, Failure)]
pub(crate) enum BooksError {
    /// The catalogue has no book with these ids. Each id appears once, in
    /// the order the caller gave it. The field `books` is a list even for
    /// one id, so an agent reads one shape for the code.
    #[failure(
        code = NOT_FOUND,
        recovery = Recovery::Run(Call::new(&["books", "list"])),
        field(books = FieldValue::list(ids.iter().copied())),
    )]
    NotFound {
        /// The ids that name no book.
        ids: Vec<u32>,
    },
    /// The file that `--from` names cannot be read.
    #[failure(
        code = UNREADABLE_IDS_FILE,
        hint = "pass --from a file that exists and that shelf can read",
        alternative = Recovery::Run(show_call()),
    )]
    UnreadableIdsFile {
        /// The path of the file as the caller gave it.
        name: String,
        /// Why the file cannot be read.
        error: io::Error,
    },
    /// A line of the file that `--from` names is not a book id. The failure
    /// is its own diagnostic, so crumbkit prints the line with a label.
    #[failure(
        code = BAD_IDS_FILE,
        hint = "each line of the ids file holds one book id",
        alternative = Recovery::Run(show_call()),
        diagnostic,
    )]
    BadIdsFile {
        /// The file's text, named by its path as the caller gave it.
        file: NamedSource<String>,
        /// The span of the first line's text that is not a book id.
        span: SourceSpan,
        /// Why the text is not a book id, as the id parser says it.
        problem: String,
    },
    /// The file that `--from` names holds more than [`MOST_IDS`] ids. The
    /// failure is its own diagnostic, so crumbkit prints the first id after
    /// the last one allowed with a label.
    #[failure(
        code = TOO_MANY_IDS,
        hint = "the ids file holds at most {MOST_IDS} ids, as the command line does",
        alternative = Recovery::Run(show_call()),
        diagnostic,
    )]
    TooManyIds {
        /// The file's text up to the line after the first id after the last
        /// one allowed, named by its path as the caller gave it.
        file: NamedSource<String>,
        /// The span of the first id after the last one allowed.
        span: SourceSpan,
    },
    /// The file that `--from` names holds no id.
    #[failure(
        code = EMPTY_IDS_FILE,
        hint = "the ids file needs at least one book id",
        alternative = Recovery::Run(show_call()),
    )]
    EmptyIdsFile {
        /// The path of the file as the caller gave it.
        name: String,
    },
    /// The file of removed books cannot be read or written, or holds a line
    /// that is not a book id.
    #[failure(
        code = REMOVED_FILE,
        hint = "make {name} a readable file that holds one book id per line, or delete it",
    )]
    RemovedFile {
        /// The path of the file.
        name: String,
        /// Why the file cannot be read or written.
        error: io::Error,
    },
    /// Neither `XDG_STATE_HOME` nor `HOME` names a directory, so shelf has
    /// nowhere to keep the removed books.
    #[failure(code = NO_STATE_DIRECTORY, hint = "set XDG_STATE_HOME or HOME to a directory")]
    NoStateDirectory,
}

/// Prints `the catalogue has no book 7` for one id, and `the catalogue has no
/// books 7, 9` for more. An unreadable ids file prints `cannot read the ids
/// file ids.txt`, and a bad line prints `the ids file holds a line that is
/// not a book id`, with a label that says what is wrong with the line. A
/// file with too many ids or with none names the file.
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
            Self::UnreadableIdsFile { name, .. } => write!(f, "cannot read the ids file {name}"),
            Self::BadIdsFile { .. } => {
                f.write_str("the ids file holds a line that is not a book id")
            }
            Self::TooManyIds { file, .. } => write!(
                f,
                "the ids file {} holds more than {MOST_IDS} ids",
                file.name()
            ),
            Self::EmptyIdsFile { name } => write!(f, "the ids file {name} holds no book ids"),
            Self::RemovedFile { name, .. } => {
                write!(f, "cannot read or write the file of removed books {name}")
            }
            Self::NoStateDirectory => {
                f.write_str("shelf cannot find a state directory to keep the removed books in")
            }
        }
    }
}

/// An unreadable ids file has the I/O error as its source, which prints as
/// the failure's cause.
impl std::error::Error for BooksError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnreadableIdsFile { error, .. } | Self::RemovedFile { error, .. } => Some(error),
            Self::NotFound { .. }
            | Self::BadIdsFile { .. }
            | Self::TooManyIds { .. }
            | Self::EmptyIdsFile { .. }
            | Self::NoStateDirectory => None,
        }
    }
}

/// A bad line in the ids file, and the id after the last one allowed, have
/// the file as their source code and one label on the line's text, so
/// crumbkit prints the line with an underline. The other failures have
/// neither.
impl Diagnostic for BooksError {
    fn source_code(&self) -> Option<&dyn SourceCode> {
        match self {
            Self::BadIdsFile { file, .. } | Self::TooManyIds { file, .. } => Some(file),
            Self::NotFound { .. }
            | Self::UnreadableIdsFile { .. }
            | Self::EmptyIdsFile { .. }
            | Self::RemovedFile { .. }
            | Self::NoStateDirectory => None,
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let label = match self {
            Self::BadIdsFile { span, problem, .. } => {
                LabeledSpan::new_with_span(Some(problem.clone()), *span)
            }
            Self::TooManyIds { span, .. } => LabeledSpan::new_with_span(
                Some(format!("this is id {} of the file", MOST_IDS + 1)),
                *span,
            ),
            Self::NotFound { .. }
            | Self::UnreadableIdsFile { .. }
            | Self::EmptyIdsFile { .. }
            | Self::RemovedFile { .. }
            | Self::NoStateDirectory => return None,
        };
        Some(Box::new(std::iter::once(label)))
    }
}
