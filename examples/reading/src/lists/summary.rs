//! `reading lists list`, which prints each reading list as one line with
//! its about text.

use std::path::Path;

use crumbkit::{Document, Listing, Page, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::lists_in;

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
}

/// A list prints as `NAME: ABOUT` in text, the way a person writes a list
/// of lists, and as a table with `--fields`.
impl Row for ListSummary {
    const COLUMNS: &'static [&'static str] = &["name", "about"];
    const LINE: Option<&'static str> = Some("<NAME>: <ABOUT>");

    fn text_line(&self) -> Option<String> {
        Some(format!("{}: {}", self.name, self.about))
    }
}

/// Returns the page of the lists in `dir` that `page` asks for, in name
/// order. A directory that is `None` or holds no lists gives an empty page.
pub(crate) fn summaries<const N: usize>(
    dir: Option<&Path>,
    page: &Page<N>,
) -> Listing<ListSummary> {
    let lists = dir.map(lists_in).unwrap_or_default();
    let rows = lists
        .into_iter()
        .map(|list| ListSummary {
            name: list.name,
            about: list.about,
        })
        .collect();
    page.first_of(rows).described("lists")
}
