//! reading's static command-line grammar and the guide of its static
//! commands. The commands that reading adds for its lists are in
//! `graft.rs`.

use std::num::NonZeroUsize;

use clap::{Args, Parser, Subcommand};
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Effects, Example, Guide, Leaf};
use crumbkit::pick::Choices;
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};
use crumbkit::{DryRun, Expect, NoInput, Page, ViewFlags, Yes};
use crumbkit_picker::Menu;

use crate::lists::{
    ENTRIES_EXPORT, ENTRIES_LIST, ENTRIES_SET, ENTRIES_STREAM, LISTS_LIST, LISTS_SHOW, choices,
};

/// Keep reading lists, and print the entries of each.
#[derive(Debug, Parser)]
#[command(name = "reading", version)]
pub struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    pub(crate) resource: Resource,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) view: ViewFlags,

    /// The flag that tells reading that nobody can answer a question.
    #[command(flatten)]
    pub(crate) no_input: NoInput,
}

/// The menu that asks a person at a terminal which list to read, when
/// `entries list`, `entries stream` or `entries export` names none.
static MENU: Menu = Menu::new();

/// The source of the names of reading's lists, which the menu offers for
/// `--list`.
const LISTS: &[Choices] = &[Choices::new("list", choices)];

/// The leaves of reading's static commands. reading guides no command, so
/// `reading -h` keeps clap's `Commands:` list, which also lists the command
/// of each reading list. `crate::guide` adds a leaf for each of those
/// commands at run time. A person at a terminal who leaves out `--list`
/// picks a list from the menu.
pub const GUIDE: Guide = Guide::new().picker(&MENU).leaves(&[
    Leaf::new(&["entries", "list"])
        .examples(&[Example::new(
            &["--list", "to-read"],
            "Print the first page of the to-read list",
        )])
        .effects(Effects::read_only())
        .document(ENTRIES_LIST)
        .choices(LISTS),
    Leaf::new(&["entries", "stream"])
        .examples(&[
            Example::new(
                &["--list", "to-read"],
                "Print each entry of the to-read list as reading reads it",
            ),
            Example::new(
                &["--list", "to-read", "--json"],
                "Print each entry of the to-read list as one JSON document on each line",
            ),
        ])
        .effects(Effects::read_only())
        .stream(ENTRIES_STREAM)
        .choices(LISTS),
    Leaf::new(&["entries", "export"])
        .examples(&[
            Example::new(&["--list", "to-read"], "Print the file of the to-read list"),
            Example::new(
                &["--list", "to-read", "--json"],
                "Print the file of the to-read list as JSON text, or as base64 when it is not UTF-8",
            ),
        ])
        .effects(Effects::read_only())
        .document(ENTRIES_EXPORT)
        .choices(LISTS),
    Leaf::new(&["entries", "set"])
        .examples(&[
            Example::new(
                &[
                    "--list",
                    "to-read",
                    "--position",
                    "2",
                    "--expect",
                    "Wuthering Heights",
                    "--dry-run",
                    "Mary Barton",
                ],
                "Print the entry that replacing entry 2 of the to-read list would write",
            ),
            Example::new(
                &[
                    "--list",
                    "to-read",
                    "--position",
                    "6",
                    "--expect",
                    "none",
                    "--yes",
                    "Emma",
                ],
                "Append Emma to the to-read list without asking",
            ),
        ])
        .effects(Effects::replacing())
        .document(ENTRIES_SET),
    Leaf::new(&["lists", "list"])
        .examples(&[
            Example::new(&[], "Print each reading list as one line with its about text"),
            Example::new(&["--fields", "name"], "Print the name of each reading list as a table"),
        ])
        .effects(Effects::read_only())
        .document(LISTS_LIST),
    Leaf::new(&["lists", "show"])
        .examples(&[
            Example::new(
                &["to-read", "finished"],
                "Print the files of the to-read and finished lists, each under a header",
            ),
            Example::new(
                &["to-read", "--json"],
                "Print the file of the to-read list as JSON, with its text or base64 beside its name",
            ),
        ])
        .effects(Effects::read_only())
        .document(LISTS_SHOW),
    Leaf::new(&["commands", "list"])
        .examples(&[
            Example::new(&[], "List every command"),
            Example::new(
                &["to-read", "list", "--json", "--full"],
                "Print the list command of the to-read list with its arguments and examples as JSON",
            ),
        ])
        .effects(Effects::read_only())
        .document(COMMANDS_LIST),
    Leaf::new(&["schemas", "list"])
        .examples(&[Example::new(&[], "List every document that reading prints")])
        .effects(Effects::read_only())
        .document(SCHEMAS_LIST),
    Leaf::new(&["schemas", "show"])
        .examples(&[Example::new(
            &["reading/entries-list/v1"],
            "Print the JSON Schema of the document that each list command prints",
        )])
        .effects(Effects::read_only())
        .document(SCHEMAS_SHOW),
]);

/// The resources of reading's static grammar.
#[derive(Debug, Subcommand)]
pub(crate) enum Resource {
    /// Work with the entries of any reading list.
    Entries {
        /// What to do with the entries.
        #[command(subcommand)]
        command: EntriesCommand,
    },
    /// Print the files of reading's lists.
    Lists {
        /// What to do with the lists.
        #[command(subcommand)]
        command: ListsCommand,
    },
    /// Explore reading's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
    /// Explore the JSON documents that reading prints.
    Schemas {
        /// What to do with the documents.
        #[command(subcommand)]
        command: SchemasCommand,
    },
}

/// The commands for the entries of any reading list.
#[derive(Debug, Subcommand)]
pub(crate) enum EntriesCommand {
    /// List the entries of the reading list that --list names, in file order.
    List(EntriesList),
    /// Print each entry of the reading list that --list names as reading reads it.
    Stream(EntriesStream),
    /// Replace or append an entry of the reading list that --list names.
    Set(EntriesSet),
    /// Print the file of the reading list that --list names, as the file holds it.
    Export(EntriesExport),
}

/// The commands for reading's lists as a whole.
#[derive(Debug, Subcommand)]
pub(crate) enum ListsCommand {
    /// List reading's lists, each as one line with its about text.
    List(ListsList),
    /// Print the file of each reading list named, each under a header when there are several.
    Show(ListsShow),
}

/// The commands for reading's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of reading's commands with their arguments and examples.
    List(CommandsList),
}

/// The commands for the JSON documents that reading prints.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemasCommand {
    /// List the JSON documents that reading prints, with a description of each.
    List(SchemasList),
    /// Print the JSON Schema of each document.
    Show(SchemasShow),
}

/// The arguments of `reading entries list`.
#[derive(Debug, Args)]
pub(crate) struct EntriesList {
    /// The name of the reading list, such as to-read
    #[arg(long, value_name = "NAME")]
    pub(crate) list: String,

    /// How many entries to print.
    #[command(flatten)]
    pub(crate) page: Page<3>,
}

/// The arguments of `reading entries stream`.
#[derive(Debug, Args)]
pub(crate) struct EntriesStream {
    /// The name of the reading list, such as to-read
    #[arg(long, value_name = "NAME")]
    pub(crate) list: String,
}

/// The arguments of `reading entries export`.
#[derive(Debug, Args)]
pub(crate) struct EntriesExport {
    /// The name of the reading list, such as to-read
    #[arg(long, value_name = "NAME")]
    pub(crate) list: String,
}

/// The arguments of `reading lists list`.
#[derive(Debug, Args)]
pub(crate) struct ListsList {
    /// How many lists to print.
    #[command(flatten)]
    pub(crate) page: Page<10>,
}

/// The arguments of `reading lists show`.
#[derive(Debug, Args)]
pub(crate) struct ListsShow {
    /// The names of the reading lists, such as to-read
    #[arg(required = true, num_args = 1..=100, value_name = "NAME")]
    pub(crate) names: Vec<String>,
}

/// The arguments of `reading entries set`.
#[derive(Debug, Args)]
pub(crate) struct EntriesSet {
    /// The name of the reading list, such as to-read
    #[arg(long, value_name = "NAME")]
    pub(crate) list: String,

    /// The entry's position from 1, where one past the last appends
    #[arg(long, value_name = "N")]
    pub(crate) position: NonZeroUsize,

    /// The title of the book
    #[arg(value_name = "TITLE", value_parser = title)]
    pub(crate) title: String,

    /// The title that the entry has now, which is the entry's version.
    #[command(flatten)]
    pub(crate) expect: Expect,

    /// Print the entry that the command would write, and write nothing.
    #[command(flatten)]
    pub(crate) dry_run: DryRun,

    /// Write the entry without asking.
    #[command(flatten)]
    pub(crate) yes: Yes,
}

/// Returns `value` as a title, or refuses a value that is empty, holds a
/// line break or starts with `#`, because a list's file holds one entry on
/// each line and reads a line that starts with `#` as a comment.
fn title(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err("a title cannot be empty".to_owned())
    } else if value.contains(['\n', '\r']) {
        Err("a title cannot hold a line break".to_owned())
    } else if value.trim_start().starts_with('#') {
        Err("a title cannot start with #, which marks a comment in a list's file".to_owned())
    } else {
        Ok(value.trim().to_owned())
    }
}
