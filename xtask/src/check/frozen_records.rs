//! The frozen record check (DOCS-6). A decision record whose version at the
//! merge base has a `- Date:` line is frozen. The branch may change its
//! Status line and, in a node record, its `## Children` section. Any other
//! change, or removing the record, is reported. Records are matched by
//! number, so a record may move to another directory.

use std::collections::BTreeMap;

use super::adr_records::{is_record, title_number};
use super::{File, Violation};

#[cfg(test)]
mod tests;

/// Reports each frozen record in `base` that `head` edits or removes.
pub(super) fn check(base: &[File], head: &[File]) -> Vec<Violation> {
    let head_records: BTreeMap<u16, &File> = records(head).collect();

    let mut violations = Vec::new();
    for (number, old) in records(base) {
        if !old.contents.lines().any(|line| line.starts_with("- Date:")) {
            continue;
        }
        match head_records.get(&number) {
            Some(new) if frozen_text(&new.contents) == frozen_text(&old.contents) => {}
            Some(new) => violations.push(Violation::EditedRecord {
                path: new.path.clone(),
                number,
            }),
            None => violations.push(Violation::EditedRecord {
                path: old.path.clone(),
                number,
            }),
        }
    }
    violations
}

/// Returns each record in `files` with its number. Files without a record
/// title are left to the decision record check.
fn records(files: &[File]) -> impl Iterator<Item = (u16, &File)> {
    files
        .iter()
        .filter(|file| is_record(&file.path))
        .filter_map(|file| Some((title_number(&file.contents)?, file)))
}

/// Returns the lines of `contents` that may not change: every line except
/// Status lines and the `## Children` section, which runs from its heading
/// to the next `## ` heading or the end of the file.
fn frozen_text(contents: &str) -> Vec<&str> {
    let mut in_children = false;
    let mut lines = Vec::new();
    for line in contents.lines() {
        if line.starts_with("## ") {
            in_children = line == "## Children";
        }
        if !in_children && !line.starts_with("- Status:") {
            lines.push(line);
        }
    }
    lines
}
