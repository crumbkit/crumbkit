//! shelf's command-line grammar.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};
use crumbkit::usage::{SELF_USAGE, SelfUsage};
use crumbkit::{Output, Page};
use crumbkit_aliases::AliasesList;
use schemars::JsonSchema;
use serde::Serialize;

use crate::books::{BOOKS_LIST, BOOKS_SHOW, MOST_IDS};

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
/// root, so `shelf -h` lists `books` under `Browse the catalogue`,
/// `commands` and `schemas` under `Explore the commands` and `self` under
/// `Review how shelf is used`, and `shelf books -h` keeps clap's
/// `Commands:` list. The help of each leaf lists its examples, and its
/// `--help` describes the columns of the rows it prints.
/// `shelf help catalogue` prints `cli/catalogue.txt`.
pub const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(
        &[],
        &[
            Goal::new("Browse the catalogue", &["books"]),
            Goal::new("Explore the commands", &["commands", "schemas"]),
            Goal::new("Review how shelf is used", &["self"]),
        ],
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
        Leaf::new(&["commands", "list"])
            .examples(&[
                Example::new(&[], "List every command"),
                Example::new(
                    &["books"],
                    "List the books command and the commands below it",
                ),
                Example::new(
                    &["books", "list", "--json", "--full"],
                    "Print books list with its arguments, examples and columns as JSON",
                ),
            ])
            .document(COMMANDS_LIST),
        Leaf::new(&["schemas", "list"])
            .examples(&[Example::new(&[], "List every document that shelf prints")])
            .document(SCHEMAS_LIST),
        Leaf::new(&["schemas", "show"])
            .examples(&[Example::new(
                &["shelf/books-list/v1"],
                "Print the JSON Schema of the document that books list prints",
            )])
            .document(SCHEMAS_SHOW),
        Leaf::new(&["self", "usage"])
            .examples(&[
                Example::new(&[], "Report the findings of the last 7 days"),
                Example::new(&["--since", "1d"], "Report the findings of the last day"),
            ])
            .document(SELF_USAGE),
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
    /// Explore shelf's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
    /// Explore the JSON documents that shelf prints.
    Schemas {
        /// What to do with the documents.
        #[command(subcommand)]
        command: SchemasCommand,
    },
    /// Review shelf's own use.
    #[command(name = "self")]
    SelfCommand {
        /// What to review.
        #[command(subcommand)]
        command: SelfCommand,
    },
    /// Work with a person's aliases for shelf's commands.
    // The group is hidden, because aliases are for people, and an agent
    // reads only the real commands.
    #[command(hide = true)]
    Aliases {
        /// What to do with the aliases.
        #[command(subcommand)]
        command: AliasesCommand,
    },
}

/// The commands for books.
#[derive(Debug, Subcommand)]
pub(crate) enum BooksCommand {
    /// List the books in the catalogue, in id order.
    // `ls` is a hidden alias, so a person can type it, and help, `Next:`
    // lines and `commands list` name only `list`.
    #[command(alias = "ls")]
    List(BooksList),
    /// Show the books with the given ids, in the order given.
    Show(BooksShow),
}

/// The commands for shelf's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of shelf's commands with their arguments, examples and columns.
    List(CommandsList),
}

/// The commands for the JSON documents that shelf prints.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemasCommand {
    /// List the JSON documents that shelf prints, with a description of each.
    List(SchemasList),
    /// Print the JSON Schema of each document.
    Show(SchemasShow),
}

/// The commands that review shelf itself.
#[derive(Debug, Subcommand)]
pub(crate) enum SelfCommand {
    /// Report the patterns in shelf's usage log where callers waste effort.
    Usage(SelfUsage),
}

/// The commands for a person's aliases.
#[derive(Debug, Subcommand)]
pub(crate) enum AliasesCommand {
    /// List the built-in aliases and the aliases in shelf's aliases file.
    List(AliasesList),
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

/// The arguments of `shelf books show`: the ids on the command line, or a
/// file that holds them.
#[derive(Debug, Args)]
pub(crate) struct BooksShow {
    /// The ids of the books to show, separated by spaces
    // The ids have no `value_delimiter = ','`. clap's `num_args` counts the
    // values on the command line before it splits them at a delimiter, so
    // `2,5` would count as one value and a comma-separated list could pass
    // more than 100 ids.
    //
    // RUST-10: the ids are required and conflict with `--from`, so the usage
    // line shows `<ID>...` although `--from` alone is enough. clap does not
    // require an argument when an argument it conflicts with is given. The
    // `one_of` field of `commands list` names `--from` on the ids, so a reader
    // of the listing learns that `--from` can be given in their place. A
    // required argument group of the ids and `--from` would show this in the
    // usage line, but clap 4.6 prints that group as `<ID|--from <FILE>>`.
    // The group replaces this once clap prints a group of a positional
    // argument and an option in a form that reads as clearly as `<ID>...`.
    #[arg(
        required = true,
        value_name = "ID",
        num_args = 1..=MOST_IDS,
        conflicts_with = "from"
    )]
    pub(crate) ids: Vec<u32>,

    /// Read the ids from this file, one id per line
    #[arg(long, value_name = "FILE")]
    pub(crate) from: Option<PathBuf>,
}
