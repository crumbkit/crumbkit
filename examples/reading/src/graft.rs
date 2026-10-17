//! The commands that reading adds at run time, one for each list, and the
//! parsed form of a command line that may name one of them.
//!
//! reading adds each list's command to the tree before parsing. The tree
//! must hold every command when `crumbkit::parse_from` builds the help
//! lines, and when crumbkit-test's checks walk it, so reading cannot add the
//! commands with clap's `Command::defer`, which runs only when clap builds
//! the tree.

use clap::{ArgMatches, Args, Command, CommandFactory as _, FromArgMatches, Subcommand};
use crumbkit::Page;
use crumbkit::help::{Effects, Guide, RuntimeLeaf};

use crate::cli::{Cli, GUIDE, Resource};
use crate::lists::{ENTRIES_LIST, List};

#[cfg(test)]
mod tests;

/// The commands of one list, below the list's own command.
#[derive(Debug, Subcommand)]
pub(crate) enum ListCommand {
    /// List the entries of this reading list, in file order.
    List(ListEntries),
    /// Open this reading list in your editor: $VISUAL, $EDITOR or vi.
    Edit,
}

/// The arguments of `reading <name> list`.
#[derive(Debug, Args)]
pub(crate) struct ListEntries {
    /// How many entries to print.
    #[command(flatten)]
    pub(crate) page: Page<3>,
}

/// A parsed command line: a command of reading's static grammar, or the
/// command of a list with the list's name.
#[derive(Debug)]
pub(crate) enum Parsed {
    /// A command of the static grammar.
    Static(Cli),
    /// A command of the list named `name`.
    Grafted {
        /// The name of the list.
        name: String,
        /// What to do with the list.
        command: ListCommand,
    },
}

/// A subcommand that the static grammar does not hold is a list's command,
/// because reading skips a list whose name the static grammar holds.
impl FromArgMatches for Parsed {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        match matches.subcommand() {
            Some((name, list)) if !Resource::has_subcommand(name) => Ok(Self::Grafted {
                name: name.to_owned(),
                command: ListCommand::from_arg_matches(list)?,
            }),
            _ => Cli::from_arg_matches(matches).map(Self::Static),
        }
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

/// Returns reading's command tree: the static grammar, with a command for
/// each of `lists` after the static commands. Each list's command has the
/// list's about text and two subcommands, `list` and `edit`.
#[must_use]
pub fn command(lists: &[List]) -> Command {
    lists.iter().fold(Cli::command(), |command, list| {
        // `augment_subcommands` sets the about text of `ListCommand`'s doc
        // comment, so the list's about text is set after it.
        let grafted = ListCommand::augment_subcommands(Command::new(list.name().to_owned()))
            .about(list.about().to_owned())
            .subcommand_required(true)
            .arg_required_else_help(true);
        command.subcommand(grafted)
    })
}

/// Returns reading's guide: [`GUIDE`], with a leaf that lists two examples
/// for the `list` command of each of `lists` and declares the document it
/// prints, the one that `entries list` prints, and a leaf with one example
/// for its `edit` command, which passes on the editor's exit code. With no
/// lists it returns [`GUIDE`] itself and allocates nothing.
#[must_use]
pub fn guide(lists: &[List]) -> Guide {
    if lists.is_empty() {
        return GUIDE;
    }
    let leaves = lists
        .iter()
        .flat_map(|list| {
            let name = list.name();
            let list_leaf = RuntimeLeaf::new(vec![name.to_owned(), "list".to_owned()])
                .example(
                    Vec::new(),
                    format!("Print the first page of the {name} list"),
                )
                .example(
                    vec!["--all".to_owned()],
                    format!("Print every entry of the {name} list"),
                )
                .effects(Effects::read_only())
                .document(ENTRIES_LIST);
            // The editor can delete or overwrite any line of the list, and
            // it saves the whole file over the one it opened.
            let edit_leaf = RuntimeLeaf::new(vec![name.to_owned(), "edit".to_owned()])
                .example(Vec::new(), format!("Open the {name} list in your editor"))
                .effects(Effects::replacing())
                .passes_on_exit("the editor");
            [list_leaf, edit_leaf]
        })
        .collect();
    GUIDE.with_runtime_leaves(leaves)
}
