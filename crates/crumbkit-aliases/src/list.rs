//! `<tool> aliases list`, which lists the built-in aliases of the command
//! tree and the entries of every source, with the problem of each entry that
//! does not work.

use clap::{Args, Command};
use crumbkit::help::quote;
use crumbkit::{Document, Listing, Page, Problem, Row, Run};
use schemars::JsonSchema;
use serde::Serialize;

use crate::Aliases;
use crate::judge::{Content, judge};
use crate::source::Source;

#[cfg(test)]
mod tests;

/// The document that `aliases list` prints: one page of alias rows and the
/// number of aliases. Its schema is
/// `crates/crumbkit-aliases/schemas/aliases-list-v1.json`.
pub const ALIASES_LIST: Document<Listing<AliasRow>> = Document::new(
    "crumbkit",
    "aliases-list",
    1,
    "The built-in aliases and the entries of every source, with the problem of each entry that does not work.",
)
.built_in();

/// The arguments of `<tool> aliases list`: the paging flags.
///
/// A tool's hidden `aliases list` subcommand takes it as its arguments
/// type, as in the [crate example](crate).
#[derive(Args, Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct AliasesList {
    /// How many aliases to print.
    #[command(flatten)]
    #[serde(flatten)]
    page: Page,
}

/// One built-in alias or source entry, as `aliases list` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct AliasRow {
    /// The words that a person types after the tool's name, such as `books ls` for a built-in alias or `mine` for an entry of a source.
    name: String,
    /// The command that the alias stands for, without the tool's name, such as `books list`. An entry's words are quoted for a POSIX shell, and the command is empty when the entry's value is not a list of words.
    command: String,
    /// Why the entry does not expand, or empty when the alias works.
    problem: String,
    /// Where the alias comes from: the tool's name for a built-in alias, or the path of the file or the origin of the source for an entry.
    from: String,
}

impl Row for AliasRow {
    // Every field is a compact column. A person runs the command to see what
    // each alias stands for and why an entry does not work, so the problem
    // shows without `--long`. The source comes last because a file's path is
    // the longest cell, and the columns before it stay aligned.
    const COLUMNS: &'static [&'static str] = &["name", "command", "problem", "from"];
    // An entry that does not work makes `aliases list` exit with 1, so a
    // check of the aliases file fails on it.
    const PROBLEM: Option<Problem<Self>> = Some(Problem::field("problem"));
}

impl Aliases {
    /// Returns the built-in aliases of the command tree that `run` holds and
    /// the entries of every source, one page of them as `args` asks, for the
    /// tool to pass to [`finish`](crumbkit::finish) with [`ALIASES_LIST`].
    ///
    /// The entries with a problem come first, so a page cut short by a limit
    /// still shows every problem it can (OUTPUT-3). The built-in aliases
    /// follow, in the depth-first order of the visible tree. A built-in alias
    /// is a clap alias, hidden or visible, of a visible subcommand, and its
    /// name is its full path below the root, such as `books ls` for the alias
    /// `ls` of `books list`. A hidden command and everything below it are left
    /// out, and so are the aliases of arguments and the short and long flag
    /// aliases of flag subcommands. The entries of the sources come last,
    /// each source in the order the tool added it and the entries of one
    /// source in name order. Each entry has the problem that keeps it from
    /// expanding, as [`expand`](Aliases::expand) decides it, and the entries
    /// with a problem keep that order among themselves. The `from` column
    /// names the source: the path of a file, or the origin that the tool
    /// gave.
    ///
    /// A source that cannot be read or parsed has one row with its origin as
    /// the name and the reason as the problem, so the command exits with 1
    /// and the other sources still list.
    ///
    /// It builds the whole command tree through `Run::command`, so it sees
    /// each command that `Command::defer` adds.
    ///
    /// A missing file, or no file because the environment gives no config
    /// directory, lists the other aliases only. The listing's note, which the
    /// text output prints on standard error after the table, names each file
    /// and says when it does not exist yet, or says why the tool has no
    /// aliases file.
    #[must_use]
    pub fn list(&self, run: &mut Run, args: &AliasesList) -> Listing<AliasRow> {
        self.listing(run.command(), args)
    }

    /// Returns the page of rows that `args` asks for, with the note that
    /// names the files, in the order that [`Aliases::rows`] gives.
    pub(crate) fn listing(&self, command: &Command, args: &AliasesList) -> Listing<AliasRow> {
        let rows = self.rows(command);
        let listing = args.page.first_of(rows).described("aliases");
        match self.note(command.get_name()) {
            Some(note) => listing.note(note),
            None => listing,
        }
    }

    /// Returns the note that names each aliases file, or `None` when no
    /// source is a file.
    fn note(&self, tool: &str) -> Option<String> {
        let notes: Vec<String> = self
            .sources()
            .iter()
            .filter_map(Source::file_state)
            .map(|(path, exists)| match (path, exists) {
                (Some(path), true) => format!("Aliases file: {path}"),
                (Some(path), false) => format!("Aliases file: {path}, which does not exist yet"),
                (None, _) => {
                    format!(
                        "{tool} has no aliases file, because the home directory cannot be found."
                    )
                }
            })
            .collect();
        (!notes.is_empty()).then(|| notes.join("\n"))
    }

    /// Returns every row: the entries with a problem, then the built-in
    /// aliases of `command`, then the entries without a problem. The entries
    /// are in source order, and in name order within a source.
    fn rows(&self, command: &Command) -> Vec<AliasRow> {
        let tool = command.get_name();
        let mut rows = Vec::new();
        built_in(command, &[], tool, &mut rows);
        let sources: Vec<&Source> = self.sources().iter().collect();
        for (source, content) in sources.iter().zip(judge(command, &sources)) {
            let from = source.origin();
            match content {
                Content::Missing => {}
                Content::Unreadable(reason) => rows.push(AliasRow {
                    name: from.clone(),
                    command: String::new(),
                    problem: format!("cannot be read: {reason}"),
                    from,
                }),
                Content::Items(mut items) => {
                    // A `DeTable` keeps keys in name order unless some crate in
                    // the build enables toml's `preserve_order` feature, which
                    // keeps file order, so the rows sort here.
                    items.sort_by_key(|judged| judged.item.name());
                    for judged in items {
                        let item = judged.item;
                        rows.push(AliasRow {
                            name: item.name(),
                            command: item.words.as_ref().map_or_else(String::new, |words| {
                                join(&words.iter().map(String::as_str).collect::<Vec<_>>())
                            }),
                            problem: judged.problem.unwrap_or_default(),
                            from: from.clone(),
                        });
                    }
                }
            }
        }
        // A stable sort keeps the order above within each group (OUTPUT-3).
        rows.sort_by_key(|row| row.problem.is_empty());
        rows
    }
}

/// Adds a row to `rows` for each alias of each visible subcommand of
/// `command`, which sits at `path` below the root, and then for the visible
/// commands below each one, in depth-first order.
fn built_in(command: &Command, path: &[&str], tool: &str, rows: &mut Vec<AliasRow>) {
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() {
            continue;
        }
        for alias in subcommand.get_all_aliases() {
            rows.push(AliasRow {
                name: join(&[path, &[alias]].concat()),
                command: join(&[path, &[subcommand.get_name()]].concat()),
                problem: String::new(),
                from: tool.to_owned(),
            });
        }
        let below = [path, &[subcommand.get_name()]].concat();
        built_in(subcommand, &below, tool, rows);
    }
}

/// Joins `words` with spaces, quoting each word that a POSIX shell would
/// split or expand.
fn join(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| quote(word))
        .collect::<Vec<_>>()
        .join(" ")
}
