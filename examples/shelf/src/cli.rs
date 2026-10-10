//! shelf's command-line grammar.

use clap::{Args, Parser, Subcommand};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};
use crumbkit::{Output, Page};
use schemars::JsonSchema;
use serde::Serialize;

use crate::books::{BOOKS_LIST, BOOKS_SHOW};

#[cfg(test)]
mod tests;

/// Browse a small catalogue of public-domain books.
#[derive(Debug, Parser)]
#[command(name = "shelf", version = env!("SHELF_VERSION"))]
pub struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    pub(crate) resource: Resource,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) output: Output,
}

/// shelf's goal headings, leaves and help topics. shelf guides only its
/// root, so `shelf -h` lists `books` under `Browse the catalogue`, and
/// `shelf books -h` keeps clap's `Commands:` list. The help of
/// `shelf books list` and `shelf books show` lists their examples, and
/// their `--help` describes the columns of the books they print.
/// `shelf help catalogue` prints `cli/catalogue.txt`.
pub const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(
        &[],
        &[Goal::new("Browse the catalogue", &["books"])],
    )])
    .leaves(&[
        Leaf::new(&["books", "list"])
            .examples(&[
                Example::new(&[], "Print the first page of books"),
                Example::new(&["--all"], "Print every book"),
            ])
            .document(BOOKS_LIST),
        Leaf::new(&["books", "show"])
            .examples(&[Example::new(&["2", "5"], "Show books 2 and 5")])
            .document(BOOKS_SHOW),
    ])
    .topics(&[Topic::new(
        "catalogue",
        "Where shelf's books come from and how their ids work",
        include_str!("cli/catalogue.txt"),
    )]);

/// The resources that shelf serves.
#[derive(Debug, Subcommand)]
pub(crate) enum Resource {
    /// Work with the books in the catalogue.
    Books {
        /// What to do with the books.
        #[command(subcommand)]
        command: BooksCommand,
    },
}

/// The commands for books.
#[derive(Debug, Subcommand)]
pub(crate) enum BooksCommand {
    /// List the books in the catalogue, in id order.
    List(BooksList),
    /// Show the books with the given ids, in the order given.
    Show(BooksShow),
}

/// The arguments of `shelf books list`.
///
/// A page holds 4 books, fewer than the catalogue's 6, so the default output
/// ends with crumbkit's footer and `--all` has books to add.
#[derive(Debug, Args, Serialize, JsonSchema)]
pub(crate) struct BooksList {
    /// How many books to print.
    #[command(flatten)]
    #[serde(flatten)]
    pub(crate) page: Page<4>,
}

/// The arguments of `shelf books show`.
#[derive(Debug, Args, Serialize, JsonSchema)]
pub(crate) struct BooksShow {
    /// The ids of the books to show, separated by spaces
    // The ids have no `value_delimiter = ','`. clap's `num_args` counts the
    // values on the command line before it splits them at a delimiter, so
    // `2,5` would count as one value and a comma-separated list could pass
    // more than 100 ids.
    #[arg(required = true, value_name = "ID", num_args = 1..=100)]
    pub(crate) ids: Vec<u32>,
}
