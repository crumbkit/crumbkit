//! The decision record check. Every Markdown file under `docs/decisions/` is a
//! record whose first line is `# ADR-NNNN. Title`, no two records use one
//! number, every `ADR-NNNN` that a file in the repository cites names a
//! record, and no file outside `docs/decisions/` cites a superseded record.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The text that starts every citation, followed by four digits.
const PREFIX: &str = "ADR-";

/// The start of the Status line of a record that a later record replaced.
const SUPERSEDED: &str = "- Status: superseded by ";

/// Reports records without a title, records that reuse a number, citations
/// of numbers that no record uses, and citations of superseded records from
/// files outside `docs/decisions/`.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    let mut violations = Vec::new();

    let mut records: BTreeMap<u16, &str> = BTreeMap::new();
    let mut superseded = BTreeSet::new();
    for file in files.iter().filter(|file| is_record(&file.path)) {
        let Some(number) = title_number(&file.contents) else {
            violations.push(Violation::MissingRecordTitle {
                path: file.path.clone(),
            });
            continue;
        };
        if let Some(first) = records.get(&number) {
            violations.push(Violation::DuplicateRecord {
                path: file.path.clone(),
                number,
                first: (*first).to_owned(),
            });
        } else {
            records.insert(number, &file.path);
        }
        if file
            .contents
            .lines()
            .any(|line| line.starts_with(SUPERSEDED))
        {
            superseded.insert(number);
        }
    }

    for file in files {
        let in_records = Path::new(&file.path).starts_with("docs/decisions");
        for number in citations(&file.contents) {
            if !records.contains_key(&number) {
                violations.push(Violation::UnknownRecord {
                    path: file.path.clone(),
                    number,
                });
            } else if !in_records && superseded.contains(&number) {
                violations.push(Violation::SupersededCitation {
                    path: file.path.clone(),
                    number,
                });
            }
        }
    }
    violations
}

/// Returns whether `path` is a Markdown file anywhere under `docs/decisions/`.
pub(super) fn is_record(path: &str) -> bool {
    let path = Path::new(path);
    path.starts_with("docs/decisions") && path.extension() == Some(OsStr::new("md"))
}

/// Returns the record number from a first line such as `# ADR-0017. Title`.
pub(super) fn title_number(contents: &str) -> Option<u16> {
    let rest = contents.lines().next()?.strip_prefix("# ")?;
    let number = number_after_prefix(rest)?;
    rest.get(PREFIX.len() + 4..)?
        .starts_with(". ")
        .then_some(number)
}

/// Returns every record number that `contents` cites.
pub(super) fn citations(contents: &str) -> impl Iterator<Item = u16> {
    contents
        .match_indices(PREFIX)
        .filter_map(|(start, _)| number_after_prefix(&contents[start..]))
}

/// Returns the number in text that starts with `ADR-` and exactly four digits.
/// A fifth digit means the text is not a citation.
fn number_after_prefix(text: &str) -> Option<u16> {
    let rest = text.strip_prefix(PREFIX)?;
    let digits = rest.get(..4)?;
    let four_digits = digits.bytes().all(|byte| byte.is_ascii_digit());
    let fifth_digit = rest.as_bytes().get(4).is_some_and(u8::is_ascii_digit);
    if !four_digits || fifth_digit {
        return None;
    }
    digits.parse().ok()
}
