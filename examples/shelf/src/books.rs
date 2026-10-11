//! The built-in book catalogue.

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::num::NonZeroUsize;
use std::path::Path;

use crumbkit::{Batch, Call, Code, Document, Failure, Listing, Recovery, Row};
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode, SourceSpan};
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
/// repeated id returns its book again. The ids come from the command line,
/// or from the file that `--from` names, as [`read_ids`] reads it.
///
/// The batch holds one `BooksError::NotFound` naming every id that the
/// catalogue lacks, each once and in the order given, and no failure when
/// every id names a book.
///
/// # Errors
///
/// Returns `BooksError::UnreadableIdsFile` or `BooksError::BadIdsFile` when
/// the ids file cannot be read or holds a line that is not a book id.
pub(crate) fn show(args: &BooksShow) -> Result<Batch<Book, BooksError>, BooksError> {
    let ids = match &args.from {
        Some(path) => read_ids(path)?,
        None => args.ids.clone(),
    };
    let mut books = Vec::with_capacity(ids.len());
    let mut missing = Vec::new();
    for result in crumbkit::fetch_each(&ids, WORKERS, |&id| find(id)) {
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

/// Returns the ids in the file at `path`, in file order. Each line holds one
/// id, with any spaces around it, and a line that holds nothing else is
/// skipped. The file holds at least 1 and at most [`MOST_IDS`] ids, as the
/// command line does, and reading stops at the line of the id after the
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
/// file up to the line that a problem points at.
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
        if ids.len() == MOST_IDS {
            return Err(IdsFileProblem::TooMany { span });
        }
        match id.parse::<u32>() {
            Ok(id) => ids.push(id),
            Err(error) => {
                return Err(IdsFileProblem::BadLine {
                    span,
                    reason: error.to_string(),
                });
            }
        }
    }
    if ids.is_empty() {
        return Err(IdsFileProblem::Empty);
    }
    Ok(ids)
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

/// The code of a failure whose ids file cannot be read.
const UNREADABLE_IDS_FILE: Code = Code::new("shelf", "books", "unreadable_ids_file");

/// The code of a failure whose ids file holds a line that is not a book id.
const BAD_IDS_FILE: Code = Code::new("shelf", "books", "bad_ids_file");

/// The code of a failure whose ids file holds more than [`MOST_IDS`] ids.
const TOO_MANY_IDS: Code = Code::new("shelf", "books", "too_many_ids");

/// The code of a failure whose ids file holds no id.
const EMPTY_IDS_FILE: Code = Code::new("shelf", "books", "empty_ids_file");

/// A failure of a books command.
#[derive(Debug)]
pub(crate) enum BooksError {
    /// The catalogue has no book with these ids. Each id appears once, in
    /// the order the caller gave it.
    NotFound {
        /// The ids that name no book.
        ids: Vec<u32>,
    },
    /// The file that `--from` names cannot be read.
    UnreadableIdsFile {
        /// The path of the file as the caller gave it.
        name: String,
        /// Why the file cannot be read.
        error: io::Error,
    },
    /// A line of the file that `--from` names is not a book id.
    BadIdsFile {
        /// The file's text, named by its path as the caller gave it.
        file: NamedSource<String>,
        /// The span of the first line's text that is not a book id.
        span: SourceSpan,
        /// Why the text is not a book id, as the id parser says it.
        problem: String,
    },
    /// The file that `--from` names holds more than [`MOST_IDS`] ids.
    TooManyIds {
        /// The file's text up to the line of the first id after the last
        /// one allowed, named by its path as the caller gave it.
        file: NamedSource<String>,
        /// The span of the first id after the last one allowed.
        span: SourceSpan,
    },
    /// The file that `--from` names holds no id.
    EmptyIdsFile {
        /// The path of the file as the caller gave it.
        name: String,
    },
}

impl BooksError {
    /// Every code that a `BooksError` returns. Only shelf's test reads the
    /// list, which it passes to crumbkit-test's codes check.
    #[cfg(test)]
    pub(crate) const CODES: &'static [&'static Code] = &[
        &NOT_FOUND,
        &UNREADABLE_IDS_FILE,
        &BAD_IDS_FILE,
        &TOO_MANY_IDS,
        &EMPTY_IDS_FILE,
    ];
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
        }
    }
}

/// An unreadable ids file has the I/O error as its source, which prints as
/// the failure's cause.
impl std::error::Error for BooksError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnreadableIdsFile { error, .. } => Some(error),
            Self::NotFound { .. }
            | Self::BadIdsFile { .. }
            | Self::TooManyIds { .. }
            | Self::EmptyIdsFile { .. } => None,
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
            Self::NotFound { .. } | Self::UnreadableIdsFile { .. } | Self::EmptyIdsFile { .. } => {
                None
            }
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
            Self::NotFound { .. } | Self::UnreadableIdsFile { .. } | Self::EmptyIdsFile { .. } => {
                return None;
            }
        };
        Some(Box::new(std::iter::once(label)))
    }
}

/// A missing book's next step is `shelf books list`, which prints the ids
/// that exist. An ids file failure's next step is advice about the file.
impl Failure for BooksError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &NOT_FOUND,
            Self::UnreadableIdsFile { .. } => &UNREADABLE_IDS_FILE,
            Self::BadIdsFile { .. } => &BAD_IDS_FILE,
            Self::TooManyIds { .. } => &TOO_MANY_IDS,
            Self::EmptyIdsFile { .. } => &EMPTY_IDS_FILE,
        }
    }

    fn recovery(&self) -> Recovery {
        let hint = match self {
            Self::NotFound { .. } => return Recovery::Run(Call::new(&["books", "list"])),
            Self::UnreadableIdsFile { .. } => {
                "pass --from a file that exists and that shelf can read".to_owned()
            }
            Self::BadIdsFile { .. } => "each line of the ids file holds one book id".to_owned(),
            Self::TooManyIds { .. } => {
                format!("the ids file holds at most {MOST_IDS} ids, as the command line does")
            }
            Self::EmptyIdsFile { .. } => "the ids file needs at least one book id".to_owned(),
        };
        Recovery::Hint(hint)
    }

    fn diagnostic(&self) -> Option<&dyn Diagnostic> {
        match self {
            Self::BadIdsFile { .. } | Self::TooManyIds { .. } => Some(self),
            Self::NotFound { .. } | Self::UnreadableIdsFile { .. } | Self::EmptyIdsFile { .. } => {
                None
            }
        }
    }
}
