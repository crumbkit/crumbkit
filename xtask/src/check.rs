//! The repository checks. Each check is a pure function from files to
//! violations, so its tests need no file system.

mod adr_records;
mod agents_words;
mod docs_words;
mod rust_lines;

use std::fmt;

#[cfg(test)]
mod tests;

/// Which checks to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    /// Every check.
    All,
    /// Only the checks on `docs/`: the directory word check and the
    /// decision record check.
    Docs,
}

/// A file in the repository.
pub(crate) struct File {
    /// The path relative to the repository root, with `/` separators.
    pub(crate) path: String,
    /// The file's text.
    pub(crate) contents: String,
}

/// A problem that a check found in one file or directory.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Violation {
    /// A measured quantity is over its limit.
    OverLimit {
        /// The file or directory that was measured.
        path: String,
        /// The measured quantity.
        measured: usize,
        /// What the quantity counts, such as `lines`.
        unit: &'static str,
        /// The largest quantity that passes.
        limit: usize,
    },
    /// A Markdown file under `docs/decisions/` does not start with a record
    /// title.
    MissingRecordTitle {
        /// The file.
        path: String,
    },
    /// A record uses a number that an earlier record already uses.
    DuplicateRecord {
        /// The later record.
        path: String,
        /// The number both records use.
        number: u16,
        /// The earlier record.
        first: String,
    },
    /// A file cites a record number that no record uses.
    UnknownRecord {
        /// The citing file.
        path: String,
        /// The cited number.
        number: u16,
    },
}

/// Runs the selected checks over `files` and returns every violation.
pub(crate) fn run(selection: Selection, files: &[File]) -> Vec<Violation> {
    match selection {
        Selection::All => {
            let mut violations = rust_lines::check(files);
            violations.extend(agents_words::check(files));
            violations.extend(docs_words::check(files));
            violations.extend(adr_records::check(files));
            violations
        }
        Selection::Docs => {
            let mut violations = docs_words::check(files);
            violations.extend(adr_records::check(files));
            violations
        }
    }
}

/// Prints a violation as one line that starts with its path, such as
/// `docs: 5,312 words in this directory's Markdown, limit 5,000`.
impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OverLimit {
                path,
                measured,
                unit,
                limit,
            } => write!(
                f,
                "{path}: {} {unit}, limit {}",
                with_separators(*measured),
                with_separators(*limit)
            ),
            Self::MissingRecordTitle { path } => write!(
                f,
                "{path}: a decision record must start with a `# ADR-NNNN. Title` line"
            ),
            Self::DuplicateRecord {
                path,
                number,
                first,
            } => write!(f, "{path}: ADR-{number:04} is already used by {first}"),
            Self::UnknownRecord { path, number } => {
                write!(f, "{path}: cites ADR-{number:04}, which no record uses")
            }
        }
    }
}

/// Returns `number` in decimal with a comma between each group of three
/// digits, such as `5,312`.
fn with_separators(number: usize) -> String {
    let digits = number.to_string();
    let mut separated = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            separated.push(',');
        }
        separated.push(digit);
    }
    separated
}
