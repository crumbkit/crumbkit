//! shelf's command-line grammar.

use clap::{Args, Parser, Subcommand};
use crumbkit::{Output, Page};
use schemars::JsonSchema;
use serde::Serialize;

#[cfg(test)]
mod tests;

/// Browse a small catalogue of public-domain books.
#[derive(Debug, Parser)]
#[command(name = "shelf", version = env!("SHELF_VERSION"))]
pub(crate) struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    pub(crate) resource: Resource,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) output: Output,
}

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
    // RUST-10: the value parser accepts only the ids 1 to 6, so clap refuses
    // an id outside the catalogue with exit code 2. This works because the
    // catalogue is a fixed constant. A tool with real data cannot know its ids
    // when it parses, and reports a missing id as a failure, which crumbkit's
    // Failures pull request adds.
    //
    // The ids have no `value_delimiter = ','`. clap's `num_args` counts the
    // values on the command line before it splits them at a delimiter, so
    // `2,5` would count as one value and a comma-separated list could pass
    // more than 100 ids.
    #[arg(
        required = true,
        value_name = "ID",
        num_args = 1..=100,
        value_parser = clap::value_parser!(u32).range(1..=6)
    )]
    pub(crate) ids: Vec<u32>,
}
