//! The docs trailer check (DOCS-7). When the branch adds or changes an entry
//! in the Unreleased section of a `CHANGELOG.md`, a commit on the branch must
//! end its message with a `Docs-Owed` or `Docs-Swept` trailer. Every such
//! trailer must parse. Entries that the branch only removes, as a release
//! does, need no trailer.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;

use super::{Commit, File, Violation};

#[cfg(test)]
mod tests;

/// Reports each malformed trailer in `commits`, then each changelog whose
/// Unreleased section gained an entry when no commit has a valid trailer.
/// A branch without commits changed nothing, so it passes.
pub(super) fn check(base: &[File], head: &[File], commits: &[Commit]) -> Vec<Violation> {
    let mut violations = Vec::new();
    if commits.is_empty() {
        return violations;
    }
    let mut has_trailer = false;
    for commit in commits {
        for trailer in trailers(&commit.message) {
            if parses(trailer) {
                has_trailer = true;
            } else {
                violations.push(Violation::MalformedTrailer {
                    commit: commit.id.clone(),
                    trailer: trailer.to_owned(),
                });
            }
        }
    }
    if has_trailer {
        return violations;
    }

    for file in head.iter().filter(|file| is_changelog(&file.path)) {
        let old = base
            .iter()
            .find(|old| old.path == file.path)
            .map_or_else(BTreeSet::new, |old| unreleased_entries(&old.contents));
        let gained = unreleased_entries(&file.contents)
            .iter()
            .any(|entry| !old.contains(entry));
        if gained {
            violations.push(Violation::MissingDocsTrailer {
                path: file.path.clone(),
            });
        }
    }
    violations
}

/// Returns whether `path` is a changelog.
fn is_changelog(path: &str) -> bool {
    Path::new(path).file_name() == Some(OsStr::new("CHANGELOG.md"))
}

/// Returns each entry in the `## [Unreleased]` section of `contents`. An
/// entry is a line that starts with `- ` and the indented lines after it.
fn unreleased_entries(contents: &str) -> BTreeSet<String> {
    let mut entries = BTreeSet::new();
    let mut entry: Option<String> = None;
    let mut in_unreleased = false;
    for line in contents.lines() {
        if line.starts_with("## ") {
            in_unreleased = line.starts_with("## [Unreleased]");
        }
        let continues = line.starts_with(char::is_whitespace) && !line.trim().is_empty();
        if let Some(text) = entry.as_mut().filter(|_| continues) {
            text.push('\n');
            text.push_str(line);
            continue;
        }
        entries.extend(entry.take());
        if in_unreleased && line.starts_with("- ") {
            entry = Some(line.to_owned());
        }
    }
    entries.extend(entry);
    entries
}

/// Returns the docs trailer lines in the last paragraph of `message`. A
/// message with one paragraph has only a subject, so it has no trailers.
fn trailers(message: &str) -> impl Iterator<Item = &str> {
    let paragraphs: Vec<&str> = message.trim_end().split("\n\n").collect();
    let last = match paragraphs.as_slice() {
        [_, .., last] => *last,
        _ => "",
    };
    last.lines()
        .filter(|line| line.starts_with("Docs-Owed:") || line.starts_with("Docs-Swept:"))
}

/// Returns whether `trailer` is a well-formed `Docs-Owed` or `Docs-Swept`
/// line.
fn parses(trailer: &str) -> bool {
    if let Some(ids) = trailer.strip_prefix("Docs-Swept:") {
        return ids.split(',').all(|id| is_slug(id.trim()));
    }
    let Some(fields) = trailer.strip_prefix("Docs-Owed:") else {
        return false;
    };
    let fields: Vec<&str> = fields.split(';').map(str::trim).collect();
    let [id, from, to, docs] = fields.as_slice() else {
        return false;
    };
    let has_text = |field: &str, key: &str| {
        field
            .strip_prefix(key)
            .is_some_and(|text| !text.trim().is_empty())
    };
    let paths_listed = docs
        .strip_prefix("docs:")
        .is_some_and(|paths| paths.split(',').all(|path| !path.trim().is_empty()));
    is_slug(id) && has_text(from, "from:") && has_text(to, "to:") && paths_listed
}

/// Returns whether `id` is a kebab-case slug: lowercase letters and digits in
/// groups joined by single hyphens.
fn is_slug(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|group| {
            !group.is_empty()
                && group
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}
