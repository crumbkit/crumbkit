//! The repository checks. Each check is a pure function from files to
//! violations, so its tests need no file system.

mod adr_records;
mod agents_words;
mod changelog_entries;
mod docs_trailers;
mod docs_words;
mod frozen_records;
mod missed_lines;
mod readme_words;
mod record_listings;
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
    /// decision record checks.
    Docs,
}

/// A file in the repository.
pub(crate) struct File {
    /// The path relative to the repository root, with `/` separators.
    pub(crate) path: String,
    /// The file's text, or an empty string when the file is not UTF-8 text,
    /// such as an image.
    pub(crate) contents: String,
}

/// A commit on the branch, between the merge base and `HEAD`.
pub(crate) struct Commit {
    /// The abbreviated commit hash.
    pub(crate) id: String,
    /// The full commit message.
    pub(crate) message: String,
}

/// What the branch changed: the files at the merge base with `main`, and
/// the commits after it. The history checks compare these with the files
/// at `HEAD`. An empty history makes those checks pass.
#[derive(Default)]
pub(crate) struct History {
    /// The decision records and changelogs at the merge base.
    pub(crate) base: Vec<File>,
    /// The commits after the merge base.
    pub(crate) commits: Vec<Commit>,
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
    /// A record is missing from the `README.md` of the node above it.
    UnlistedRecord {
        /// The record.
        path: String,
        /// The node record that should list it.
        node: String,
    },
    /// A node record links to a file that does not exist.
    BrokenLink {
        /// The node record.
        path: String,
        /// The link target as written.
        target: String,
    },
    /// A file outside `docs/decisions/` cites a superseded record.
    SupersededCitation {
        /// The citing file.
        path: String,
        /// The cited number.
        number: u16,
    },
    /// The branch changed a frozen record outside its Status line and a
    /// node's Children list, or removed it.
    EditedRecord {
        /// The record, or its path at the merge base when it was removed.
        path: String,
        /// The record's number.
        number: u16,
    },
    /// A record new on the branch amends a record dated less than 7 days
    /// before it and has no `- Missed:` line with text.
    MissingMissedLine {
        /// The new record.
        path: String,
        /// The new record's number.
        number: u16,
        /// The number of the record it amends.
        earlier: u16,
    },
    /// A changelog's Unreleased section gained or changed an entry, and no
    /// commit on the branch ends with a docs trailer.
    MissingDocsTrailer {
        /// The changelog.
        path: String,
    },
    /// A commit message ends with a docs trailer that does not parse.
    MalformedTrailer {
        /// The commit's abbreviated hash.
        commit: String,
        /// The trailer line as written.
        trailer: String,
    },
}

/// The two docs trailer forms, as the violations print them.
const TRAILER_FORMS: &str = "`Docs-Owed: <id>; from: <old behaviour>; to: <new behaviour>; \
                             docs: <path>[, <path>...]` or `Docs-Swept: <id>[, <id>...]`";

/// Runs the selected checks over `files` and returns every violation.
/// The frozen record check and the missed line check compare `files` with
/// `history.base` for both selections. The docs trailer check also reads `history.commits` and runs
/// only for [`Selection::All`].
pub(crate) fn run(selection: Selection, files: &[File], history: &History) -> Vec<Violation> {
    let mut violations = match selection {
        Selection::All => {
            let mut violations = rust_lines::check(files);
            violations.extend(agents_words::check(files));
            violations.extend(docs_words::check(files));
            violations.extend(readme_words::check(files));
            violations.extend(changelog_entries::check(files));
            violations.extend(adr_records::check(files));
            violations.extend(record_listings::check(files));
            violations.extend(docs_trailers::check(&history.base, files, &history.commits));
            violations
        }
        Selection::Docs => {
            let mut violations = docs_words::check(files);
            violations.extend(adr_records::check(files));
            violations.extend(record_listings::check(files));
            violations
        }
    };
    violations.extend(frozen_records::check(&history.base, files));
    violations.extend(missed_lines::check(&history.base, files));
    violations
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
            Self::UnlistedRecord { path, node } => write!(f, "{path}: not listed in {node}"),
            Self::BrokenLink { path, target } => {
                write!(f, "{path}: links to {target}, which does not exist")
            }
            Self::SupersededCitation { path, number } => write!(
                f,
                "{path}: cites ADR-{number:04}, which is superseded; cite the page in docs/ \
                 that describes the behaviour, or the record that superseded it"
            ),
            Self::EditedRecord { path, number } => write!(
                f,
                "{path}: edits ADR-{number:04}, which is frozen; only its Status line and \
                 a node's Children list may change (DOCS-6)"
            ),
            Self::MissingMissedLine {
                path,
                number,
                earlier,
            } => write!(
                f,
                "{path}: ADR-{number:04} amends ADR-{earlier:04}, which is dated less than 7 \
                 days before it, so it needs a `- Missed:` line that says what ADR-{earlier:04} \
                 missed (DOCS-8)"
            ),
            Self::MissingDocsTrailer { path } => write!(
                f,
                "{path}: an Unreleased entry changed, so a commit message must end with \
                 {TRAILER_FORMS} (DOCS-7)"
            ),
            Self::MalformedTrailer { commit, trailer } => write!(
                f,
                "commit {commit}: `{trailer}` does not parse; write {TRAILER_FORMS}, \
                 where <id> is a kebab-case slug (DOCS-7)"
            ),
        }
    }
}

/// Returns `number` in decimal with a comma between each group of three
/// digits, such as `5,312`.
///
/// This groups digits by hand because xtask depends only on clap and prints
/// only unsigned counts in English. Code that needs signed or locale-aware
/// numbers should use a formatting crate such as num-format.
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
