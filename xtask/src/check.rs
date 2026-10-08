//! The repository checks. Each check is a pure function from files to
//! violations, so its tests need no file system.

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
    /// Only the docs directory word check.
    Docs,
}

/// A file in the repository.
pub(crate) struct File {
    /// The path relative to the repository root, with `/` separators.
    pub(crate) path: String,
    /// The file's text.
    pub(crate) contents: String,
}

/// A measured quantity that is over its limit.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Violation {
    /// The file or directory that was measured.
    pub(crate) path: String,
    /// The measured quantity.
    pub(crate) measured: usize,
    /// What the quantity counts, such as `lines`.
    pub(crate) unit: &'static str,
    /// The largest quantity that passes.
    pub(crate) limit: usize,
}

/// Runs the selected checks over `files` and returns every violation.
pub(crate) fn run(selection: Selection, files: &[File]) -> Vec<Violation> {
    match selection {
        Selection::All => {
            let mut violations = rust_lines::check(files);
            violations.extend(agents_words::check(files));
            violations.extend(docs_words::check(files));
            violations
        }
        Selection::Docs => docs_words::check(files),
    }
}

/// Prints a violation as `path: measured unit, limit limit`, such as
/// `docs: 5,312 words in this directory's Markdown, limit 5,000`.
impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {}, limit {}",
            self.path,
            with_separators(self.measured),
            self.unit,
            with_separators(self.limit)
        )
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
