//! `reading lists list`, which prints each reading list as one line with
//! its about text.

use std::path::Path;
use std::time::SystemTime;

use crumbkit::{Age, Document, Listing, Page, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{List, lists_in};

#[cfg(test)]
mod tests;

/// The document that `reading lists list` prints: one page of reading's
/// lists, in name order. Its text prints each list as its own line.
pub(crate) const LISTS_LIST: Document<Listing<ListSummary>> = Document::new(
    "reading",
    "lists-list",
    1,
    "One page of reading's lists, each with its about text.",
)
.when_empty("reading has no lists");

/// One reading list and its about text.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ListSummary {
    /// The list's name, which is also the name of its command.
    name: String,
    /// The list's about text, from the first line of its file.
    about: String,
    /// The time that the list's file was last modified, or null when the
    /// file system does not say.
    updated: Option<Age>,
}

/// A list prints as `NAME: ABOUT` in text, the way a person writes a list
/// of lists, and as a table with `--fields`, whose `updated` column shows a
/// compact age such as `3h`.
impl Row for ListSummary {
    const COLUMNS: &'static [&'static str] = &["name", "about", "updated"];
    const LINE: Option<&'static str> = Some("<NAME>: <ABOUT>");

    fn text_line(&self) -> Option<String> {
        Some(format!("{}: {}", self.name, self.about))
    }
}

/// Returns the page of the lists in `dir` that `page` asks for, in name
/// order, with the age of each list's file when the current time is `now`.
/// A directory that is `None` or holds no lists gives an empty page.
pub(crate) fn summaries<const N: usize>(
    dir: Option<&Path>,
    page: &Page<N>,
    now: SystemTime,
) -> Listing<ListSummary> {
    let lists = dir.map(lists_in).unwrap_or_default();
    let rows = lists.into_iter().map(|list| summary(list, now)).collect();
    page.first_of(rows).described("lists")
}

/// Returns the summary of `list` with the age of its file when the current
/// time is `now`.
fn summary(list: List, now: SystemTime) -> ListSummary {
    ListSummary {
        updated: list.modified.map(|modified| Age::since(modified, now)),
        name: list.name,
        about: list.about,
    }
}
