//! shelf's command-line grammar.

use clap::{Parser, Subcommand};
use crumbkit::Output;

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
    /// List every book in the catalogue.
    List,
}
