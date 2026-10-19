//! `shelf books remove`, which removes books from the shelf, and the file
//! that keeps the ids of the removed books.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crumbkit::{Document, Error, Output};
use etcetera::BaseStrategy as _;

use super::{Book, BooksError, CATALOGUE};
use crate::cli::BooksRemove;

#[cfg(test)]
mod tests;

/// The document that `shelf books remove` prints: the books that the run
/// removes, or would remove on a dry run.
pub(crate) const BOOKS_REMOVE: Document<Vec<Book>> = Document::new(
    "shelf",
    "books-remove",
    1,
    "The books with the given ids, which the run removes from the shelf, or would remove on a dry run.",
);

/// Returns the path of the file that keeps the ids of the removed books,
/// `shelf/removed.txt` in the person's XDG state directory, or `None` when
/// neither `XDG_STATE_HOME` nor `HOME` names a directory.
pub(crate) fn removed_file() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the file of removed books, and XDG_STATE_HOME names its directory"
    )]
    let state_home = std::env::var_os("XDG_STATE_HOME");
    removed_file_in(state_home)
}

/// Returns the path of the file of removed books when `XDG_STATE_HOME` is
/// `state_home`, as [`removed_file`] describes.
///
/// An absolute `state_home` is the state directory. Otherwise etcetera
/// finds it, and etcetera first finds the home directory, which reads the
/// user database when `HOME` is unset. Every books command reads the file,
/// so this skips that work when `XDG_STATE_HOME` names the directory.
fn removed_file_in(state_home: Option<OsString>) -> Option<PathBuf> {
    let state = match state_home.map(PathBuf::from) {
        Some(state) if state.is_absolute() => state,
        Some(_) | None => etcetera::choose_base_strategy().ok()?.state_dir()?,
    };
    Some(state.join("shelf").join("removed.txt"))
}

/// Returns the ids in the file of removed books at `path`, or no ids when
/// `path` is `None` or no file is there.
///
/// # Errors
///
/// Returns `BooksError::RemovedFile` when the file cannot be read or holds a
/// line that is not a book id.
pub(crate) fn read_removed(path: Option<&Path>) -> Result<Vec<u32>, BooksError> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let failure = |error| BooksError::RemovedFile {
        name: path.display().to_string(),
        error,
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(failure(error)),
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.parse()
                .map_err(|error| failure(io::Error::new(io::ErrorKind::InvalidData, error)))
        })
        .collect()
}

/// Writes `ids` to the file of removed books at `path`, one id per line,
/// and creates the file's directory first.
fn write_removed(path: &Path, ids: &[u32]) -> Result<(), BooksError> {
    let mut text = String::new();
    for id in ids {
        text.push_str(&id.to_string());
        text.push('\n');
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the file of removed books, which is shelf's only file write"
    )]
    let written = match path.parent() {
        Some(parent) => fs::create_dir_all(parent).and_then(|()| fs::write(path, text)),
        None => fs::write(path, text),
    };
    written.map_err(|error| BooksError::RemovedFile {
        name: path.display().to_string(),
        error,
    })
}

/// Removes the books that `args` names from the shelf by adding their ids
/// to the file at `removed`, and returns them in the order given. A
/// repeated id returns its book once.
///
/// Removing a book that is already removed changes nothing and succeeds,
/// so a repeat of the command is safe. On a dry run, which
/// [`Output::is_dry_run`] reports, it returns the same books and writes
/// nothing. It asks for confirmation with [`Output::confirm`] after it
/// checks the ids, so a failure for an unknown id comes before the
/// question.
///
/// # Errors
///
/// Returns `BooksError::NoStateDirectory` when `removed` is `None`,
/// `BooksError::NotFound` naming each id that the catalogue lacks,
/// `BooksError::RemovedFile` when the file cannot be read or written, and
/// the failures of [`Output::confirm`].
pub(crate) fn remove(
    output: &mut Output,
    args: &BooksRemove,
    removed: Option<&Path>,
) -> Result<Vec<Book>, Error> {
    let dry_run = output.is_dry_run();
    let Some(path) = removed else {
        return Err(BooksError::NoStateDirectory.into());
    };
    let mut books: Vec<Book> = Vec::new();
    let mut missing = Vec::new();
    for &id in &args.ids {
        match CATALOGUE.iter().find(|book| book.id == id) {
            Some(book) if !books.iter().any(|kept| kept.id == id) => books.push(book.clone()),
            None if !missing.contains(&id) => missing.push(id),
            Some(_) | None => {}
        }
    }
    if !missing.is_empty() {
        return Err(BooksError::NotFound { ids: missing }.into());
    }
    let mut ids = read_removed(Some(path))?;
    let noun = if books.len() == 1 { "book" } else { "books" };
    output.confirm(
        args.yes,
        &format!("Remove {} {noun} from the shelf?", books.len()),
    )?;
    if dry_run {
        return Ok(books);
    }
    for book in &books {
        if !ids.contains(&book.id) {
            ids.push(book.id);
        }
    }
    ids.sort_unstable();
    write_removed(path, &ids)?;
    Ok(books)
}
