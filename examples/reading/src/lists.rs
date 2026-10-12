//! reading's lists: the directory that holds them, the lists in it, and the
//! entries of one list.
//!
//! Each list is a UTF-8 text file named `<name>.txt`. Its first line may
//! hold the list's about text after `# `, and each other line that is not
//! blank and does not start with `#` holds one entry.

use std::fmt;
use std::fs;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Subcommand as _;
use crumbkit::pick::Choice;
use crumbkit::{Call, Code, Document, Error, Failure, Listing, Page, Recovery, Row};
use etcetera::BaseStrategy as _;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::Resource;

mod stream;
#[cfg(test)]
mod tests;

pub(crate) use self::stream::{ENTRIES_STREAM, stream_entries};

/// The document that `reading entries list` and the list command of each
/// list print: one page of a list's entries and the number of entries in
/// it.
pub(crate) const ENTRIES_LIST: Document<Listing<Entry>> = Document::new(
    "reading",
    "entries-list",
    1,
    "One page of a reading list's entries and the number of entries in it.",
);

/// The environment variable that names the directory of reading's lists in
/// place of `reading/lists` under the XDG data directory. reading's tests
/// set it to their fixture lists.
const DIR_VARIABLE: &str = "READING_DIR";

/// The environment variables that name the editor of `reading <list> edit`,
/// in the order that reading reads them.
const EDITOR_VARIABLES: [&str; 2] = ["VISUAL", "EDITOR"];

/// One reading list in reading's directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// The list's name, the stem of its file name, which is also the name of
    /// its command.
    name: String,
    /// The list's about text, which its command's help prints.
    about: String,
}

impl List {
    /// Returns a list named `name` with the about text `about`.
    #[must_use]
    pub const fn new(name: String, about: String) -> Self {
        Self { name, about }
    }

    /// Returns the list's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the list's about text.
    #[must_use]
    pub fn about(&self) -> &str {
        &self.about
    }
}

/// One entry of a reading list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct Entry {
    /// The entry's position in the list, counting from 1.
    position: usize,
    /// The title of the book.
    title: String,
}

impl Row for Entry {
    const COLUMNS: &'static [&'static str] = &["position", "title"];
}

/// Returns the directory of reading's lists: the directory that
/// `READING_DIR` names when it is set and not empty, or `reading/lists`
/// under the XDG data directory. It returns `None` when `READING_DIR` is
/// unset and the home directory cannot be found. The function touches no
/// file.
#[must_use]
pub(crate) fn dir() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns reading's environment reads, which find its lists"
    )]
    let variable = std::env::var_os(DIR_VARIABLE);
    match variable {
        Some(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => {
            let strategy = etcetera::choose_base_strategy().ok()?;
            Some(strategy.data_dir().join("reading").join("lists"))
        }
    }
}

/// Returns the command that opens the file of the list named `name` in `dir`
/// in the caller's editor. The editor is the program and arguments that
/// `VISUAL` names, or `EDITOR` when `VISUAL` is unset or blank, or `vi` when
/// both are. The list's path follows the editor's own arguments. With `dir`
/// `None`, the path is relative to the working directory.
//
// RUST-10: reading splits the variable on whitespace and runs no shell, so
// an editor whose path holds a space, or a variable that quotes its words,
// does not run as a shell would run it.
#[must_use]
pub(crate) fn editor(dir: Option<&Path>, name: &str) -> Command {
    let variable = EDITOR_VARIABLES
        .iter()
        .find_map(|name| {
            #[expect(
                clippy::disallowed_methods,
                reason = "this module owns reading's environment reads, which find the editor"
            )]
            let value = std::env::var(name).ok()?;
            (!value.trim().is_empty()).then_some(value)
        })
        .unwrap_or_else(|| "vi".to_owned());
    let mut words = variable.split_whitespace();
    #[expect(
        clippy::disallowed_methods,
        reason = "this module builds the command that `reading <list> edit` hands the terminal to"
    )]
    let mut command = Command::new(words.next().unwrap_or("vi"));
    command.args(words);
    command.arg(dir.unwrap_or(Path::new("")).join(format!("{name}.txt")));
    command
}

/// Returns the lists in `dir`, in name order, reading only the first line of
/// each file.
///
/// A list is a file named `<name>.txt` whose name is lowercase ASCII letters
/// and digits in words joined by single dashes, starting with a letter. The
/// function skips a file whose name is not, a file whose name is a command
/// of reading's static grammar or `help`, and a file that cannot be read,
/// because a command with that name would not parse or would hide another.
/// A directory that does not exist or cannot be read holds no lists.
#[must_use]
pub fn lists_in(dir: &Path) -> Vec<List> {
    let Ok(files) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut lists: Vec<List> = files
        .filter_map(Result::ok)
        .filter_map(|file| {
            let path = file.path();
            if path.extension()? != "txt" {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_owned();
            if !is_dashed(&name) || name == "help" || Resource::has_subcommand(&name) {
                return None;
            }
            let about = about(&path).ok()?;
            let about = about.unwrap_or_else(|| format!("Work with the {name} reading list"));
            Some(List::new(name, about))
        })
        .collect();
    lists.sort_by(|a, b| a.name.cmp(&b.name));
    lists
}

/// Returns the lists in reading's directory as choices for `--list`, which
/// the menu offers when `entries list` or `entries stream` names no list.
/// A directory that cannot be found holds no lists.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a picker's source returns a Result, and reading's lists_in reports no error"
)]
pub(crate) fn choices() -> Result<Vec<Choice>, Error> {
    Ok(dir().as_deref().map(choices_in).unwrap_or_default())
}

/// Returns the lists in `dir` as choices, each labelled with its name and
/// its about text.
pub(crate) fn choices_in(dir: &Path) -> Vec<Choice> {
    lists_in(dir)
        .into_iter()
        .map(|list| {
            let label = format!("{}  {}", list.name, list.about);
            Choice::new(list.name, label)
        })
        .collect()
}

/// Returns whether `name` is lowercase ASCII letters and digits in words
/// joined by single dashes, starting with a letter.
fn is_dashed(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// Returns the about text on the first line of the file at `path`, or
/// `None` when the first line does not start with `# `.
///
/// # Errors
///
/// Returns the I/O error of a file that cannot be read as UTF-8 text.
fn about(path: &Path) -> io::Result<Option<String>> {
    let mut line = String::new();
    BufReader::new(fs::File::open(path)?).read_line(&mut line)?;
    Ok(line.trim_end().strip_prefix("# ").map(str::to_owned))
}

/// Returns the page of the entries of the list named `name` in `dir` that
/// `page` asks for, with the number of entries in the list as the total.
///
/// # Errors
///
/// Returns `EntriesError::NotFound` when `dir` is `None` or holds no list
/// named `name`, and `EntriesError::Unreadable` when the list's file cannot
/// be read as UTF-8 text.
pub(crate) fn entries<const N: usize>(
    dir: Option<&Path>,
    name: &str,
    page: &Page<N>,
) -> Result<Listing<Entry>, EntriesError> {
    let text = fs::read_to_string(file(dir, name)?).map_err(|error| EntriesError::Unreadable {
        name: name.to_owned(),
        error,
    })?;
    let entries = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .enumerate()
        .map(|(index, title)| Entry {
            position: index + 1,
            title: title.to_owned(),
        })
        .collect();
    Ok(Listing::from_all(entries, page.limit()).described("entries"))
}

/// Returns the path of the file of the list named `name` in `dir`.
///
/// # Errors
///
/// Returns `EntriesError::NotFound` when `dir` is `None` or holds no list
/// named `name`.
fn file(dir: Option<&Path>, name: &str) -> Result<PathBuf, EntriesError> {
    let not_found = || EntriesError::NotFound {
        name: name.to_owned(),
    };
    let dir = dir.ok_or_else(not_found)?;
    if !lists_in(dir).iter().any(|list| list.name == name) {
        return Err(not_found());
    }
    Ok(dir.join(format!("{name}.txt")))
}

/// The code of a failure whose list does not exist.
pub const NOT_FOUND: Code = Code::new("reading", "entries", "not_found");

/// The code of a failure whose list's file cannot be read.
pub const UNREADABLE: Code = Code::new("reading", "entries", "unreadable");

/// A failure of a command that prints a list's entries.
#[derive(Debug)]
pub(crate) enum EntriesError {
    /// reading's directory holds no list with this name.
    NotFound {
        /// The name the caller gave.
        name: String,
    },
    /// The list's file cannot be read as UTF-8 text.
    Unreadable {
        /// The list's name.
        name: String,
        /// Why the file cannot be read.
        error: io::Error,
    },
}

impl EntriesError {
    /// Every code that an `EntriesError` returns, which reading's tests pass
    /// to crumbkit-test's codes and recovery checks.
    pub(crate) const CODES: &'static [&'static Code] = &[&NOT_FOUND, &UNREADABLE];
}

/// Prints `reading has no list named unread`, or `cannot read the list
/// to-read` with the I/O error as its cause.
impl fmt::Display for EntriesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "reading has no list named {name}"),
            Self::Unreadable { name, .. } => write!(f, "cannot read the list {name}"),
        }
    }
}

impl std::error::Error for EntriesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Unreadable { error, .. } => Some(error),
        }
    }
}

/// A missing list's next step is `reading commands list`, which lists the
/// command of each list. An unreadable list's next step is advice about its
/// file.
impl Failure for EntriesError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &NOT_FOUND,
            Self::Unreadable { .. } => &UNREADABLE,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NotFound { .. } => Recovery::Run(Call::new(&["commands", "list"])),
            Self::Unreadable { .. } => Recovery::Hint(
                "make the list's file a UTF-8 text file that reading can read".to_owned(),
            ),
        }
    }
}
