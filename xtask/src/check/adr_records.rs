//! The decision record check. Every Markdown file under `docs/decisions/` is a
//! record whose first line is `# ADR-NNNN. Title`, no two records use one
//! number, and every `ADR-NNNN` that a Rust or Markdown file cites names a
//! record.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The text that starts every citation, followed by four digits.
const PREFIX: &str = "ADR-";

/// Reports records without a title, records that reuse a number, and
/// citations of numbers that no record uses.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    let mut violations = Vec::new();

    let mut records: BTreeMap<u16, &str> = BTreeMap::new();
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
    }

    for file in files {
        for number in citations(&file.contents) {
            if !records.contains_key(&number) {
                violations.push(Violation::UnknownRecord {
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
fn title_number(contents: &str) -> Option<u16> {
    let rest = contents.lines().next()?.strip_prefix("# ")?;
    let number = number_after_prefix(rest)?;
    rest.get(PREFIX.len() + 4..)?
        .starts_with(". ")
        .then_some(number)
}

/// Returns every record number that `contents` cites.
fn citations(contents: &str) -> impl Iterator<Item = u16> {
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
