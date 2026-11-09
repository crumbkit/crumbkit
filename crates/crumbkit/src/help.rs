//! A tool's help: goal headings, which list a group's
//! subcommands under headings that the tool declares, such as `Read` and
//! `Change`, in place of clap's single `Commands:` list, leaves, which give
//! a command examples and describe its output columns, help topics, which
//! `<tool> help <TOPIC>` prints, and file formats, whose schemas
//! `<tool> schemas show` prints.

mod aliases;
pub(crate) mod effects;
pub(crate) mod exit;
mod format;
mod group;
pub(crate) mod leaf;
mod list;
pub(crate) mod rename;
mod root_verbs;
mod runtime;
pub(crate) mod topic;
pub(crate) mod view;
pub(crate) mod words;

use clap::Command;

pub use self::effects::{Effects, Items};
pub use self::format::FileFormat;
pub use self::group::{Goal, Group};
pub use self::leaf::{
    BYTES_LINE, Column, Example, HEADED_LINE, Leaf, STREAM_LINE, children_line, text_line,
};
pub use self::runtime::RuntimeLeaf;
pub use self::topic::Topic;
pub use self::view::ViewFlag;
pub use self::words::quote;

pub(crate) use self::group::{apply, apply_root_verbs};
pub(crate) use self::list::goal_sections;
use self::list::write_line;
use crate::Tool;

#[cfg(test)]
mod tests;

/// What a tool declares about its help pages: one [`Group`] for each
/// command whose help lists its subcommands under goals, one [`Leaf`] for
/// each command whose help shows examples and describes its columns, the
/// tool's [`Topic`]s, the [`FileFormat`]s of the files that users write for
/// the tool, the command that callers run first, the verbs directly under
/// the root, and the function that names a person's aliases on the root's
/// help.
///
/// [`Tool::help`](crate::Tool::help) takes it. A tool that sets nothing
/// here prints clap's help with crumbkit's lines.
///
/// ```
/// use crumbkit::Tool;
/// use crumbkit::help::{Example, Help, Leaf};
///
/// const TOOL: Tool = Tool::new().help(
///     Help::new()
///         .leaves(&[Leaf::new(&["books"]).examples(&[Example::new(&[], "List the books")])]),
/// );
/// assert_eq!(TOOL.get_help().get_leaves().len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Help {
    /// The commands whose help lists their subcommands under goals.
    groups: &'static [Group],
    /// The commands whose help shows examples and describes their columns.
    leaves: &'static [Leaf],
    /// The topics that `<tool> help <TOPIC>` prints and that the root's
    /// help names, in order.
    topics: &'static [Topic],
    /// The file formats that the schema commands serve, in order.
    file_formats: &'static [FileFormat],
    /// The path from the clap root to the command that callers run first,
    /// which answers `help` with the topics.
    root: &'static [&'static str],
    /// The verbs directly under the root, which root help lists under their
    /// own heading, with the tool's reason, or `None` when the tool declares
    /// none.
    root_verbs: Option<(&'static [&'static str], &'static str)>,
    /// The function that returns the names of a person's aliases, which
    /// the root's help names, or `None` when help names no aliases.
    personal_aliases: Option<fn(&Command) -> Vec<String>>,
}

impl Help {
    /// Returns help with no groups, no leaves, no topics, no file formats,
    /// no root verbs and no personal aliases, rooted at the clap root.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            groups: &[],
            leaves: &[],
            topics: &[],
            file_formats: &[],
            root: &[],
            root_verbs: None,
            personal_aliases: None,
        }
    }

    /// Sets the commands whose help lists their subcommands under goals.
    #[must_use]
    pub const fn groups(mut self, groups: &'static [Group]) -> Self {
        self.groups = groups;
        self
    }

    /// Returns the groups that [`Help::groups`](crate::help::Help::groups) set.
    /// The name has clap's `get_` prefix, because `groups` is the builder
    /// method.
    #[must_use]
    pub const fn get_groups(&self) -> &'static [Group] {
        self.groups
    }

    /// Sets the commands whose help shows examples and describes their
    /// columns.
    #[must_use]
    pub const fn leaves(mut self, leaves: &'static [Leaf]) -> Self {
        self.leaves = leaves;
        self
    }

    /// Returns the leaves that [`Help::leaves`](crate::help::Help::leaves) set.
    /// The name has clap's `get_` prefix, because `leaves` is the builder
    /// method.
    #[must_use]
    pub const fn get_leaves(&self) -> &'static [Leaf] {
        self.leaves
    }

    /// Sets the topics that `<tool> help <TOPIC>` prints. The root's help
    /// names them on its `Topics:` line in this order.
    #[must_use]
    pub const fn topics(mut self, topics: &'static [Topic]) -> Self {
        self.topics = topics;
        self
    }

    /// Returns the topics that [`Help::topics`](crate::help::Help::topics) set.
    /// The name has clap's `get_` prefix, because `topics` is the builder
    /// method.
    #[must_use]
    pub const fn get_topics(&self) -> &'static [Topic] {
        self.topics
    }

    /// Sets the file formats that users write and no command prints, such
    /// as a configuration file, which `schemas list` and `schemas show`
    /// serve after the documents of the leaves, in this order (ADR-0035).
    #[must_use]
    pub const fn file_formats(mut self, file_formats: &'static [FileFormat]) -> Self {
        self.file_formats = file_formats;
        self
    }

    /// Returns the file formats that
    /// [`Help::file_formats`](crate::help::Help::file_formats) set, which the
    /// schema commands serve. The name has clap's `get_` prefix, because
    /// `file_formats` is the builder method.
    pub(crate) const fn get_file_formats(&self) -> &'static [FileFormat] {
        self.file_formats
    }

    /// Sets the path from the clap root to the command that callers run
    /// first, for a tool that another program runs with fixed words. A
    /// cargo subcommand names its root `cargo` and sets `&["crumbkit"]`,
    /// because cargo runs `cargo crumbkit help` as `cargo-crumbkit crumbkit
    /// help`. That command then answers `help` with the topics, its help
    /// has the `Topics:` line, and every line that names a topic starts
    /// with `cargo crumbkit` (ADR-0037). A path that names no command
    /// leaves the root at the clap root.
    #[must_use]
    pub const fn root(mut self, path: &'static [&'static str]) -> Self {
        self.root = path;
        self
    }

    /// Returns the path that [`Help::root`](crate::help::Help::root) set, or an
    /// empty path for the clap root. The name has clap's `get_` prefix, because
    /// `root` is the builder method.
    #[must_use]
    pub const fn get_root(&self) -> &'static [&'static str] {
        self.root
    }

    /// Returns the path that [`Help::root`](crate::help::Help::root) set when
    /// it names a command of `command`, the tool's clap root, and otherwise the
    /// empty path.
    pub(crate) fn root_in(&self, command: &Command) -> &'static [&'static str] {
        let mut node = command;
        for &name in self.root {
            match node.get_subcommands().find(|sub| sub.get_name() == name) {
                Some(subcommand) => node = subcommand,
                None => return &[],
            }
        }
        self.root
    }

    /// Returns the command line that runs the root of `command`, the
    /// tool's clap root, such as `cargo crumbkit`, which the lines that
    /// name a topic start with.
    pub(crate) fn invocation(&self, command: &Command) -> String {
        std::iter::once(command.get_name())
            .chain(self.root_in(command).iter().copied())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for Help {
    /// Returns [`Help::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Applies the goal headings and the leaves' examples of `tool` to the
/// whole tree of `command`, the root of the tool's unbuilt tree, and
/// returns the tree.
///
/// [`parse`](crate::parse) takes the tool record and applies its goal headings
/// to the groups that the typed words pass through, and the leaf of the deepest
/// command typed, so a tool that parses with crumbkit does not call `extend`. A
/// tool that parses with plain clap calls it once before parsing, and a test
/// calls it to check the whole tree with its goal headings, such as with clap's
/// `Command::debug_assert`.
///
/// Each group's help page lists its visible subcommands under its goals'
/// headings, in the tool record's order, and then the visible subcommands that
/// no goal names under a final `Commands` heading. The root's page lists the
/// root verbs that [`Help::root_verbs`](crate::help::Help::root_verbs) declares
/// and no goal names under `Verbs`, before `Commands`, also when the tool
/// record gives the root no group. shelf's root group is compact:
/// [`shelf-h.txt`][shelf-h] shows its `-h` and [`shelf-help.txt`][shelf-help]
/// its `--help`.
///
/// Each line holds two spaces, the subcommand's name padded to the longest
/// name on the page plus two spaces, and its description, which is its
/// `about`, or its `long_about` when it has no `about`. A subcommand without
/// a description ends its line at its name. A goal leaves out a hidden
/// subcommand and a name that is not a subcommand of its group, and a goal
/// with nothing to list prints no heading. A subcommand that two goals name
/// is listed under both. The `Options:` section and crumbkit's lines from
/// [`parse`](crate::parse) follow the list in `-h` and `--help`.
///
/// A command's own `before_help` prints at the top of `-h`, and its
/// `before_long_help` at the top of `--help`, as in clap's default layout.
/// A list that holds `{` replaces them, because clap's template would expand
/// or drop that text, so a description or a heading with a brace costs the
/// command its own `before_help`.
///
/// The `-h` of a [`Group::compact`] group lists each goal on one line: the
/// heading with a colon, padded to the longest heading plus two spaces, and
/// then the goal's subcommand names separated by `, `. Its `--help` holds
/// the full list above. The two lists take the command's `before_help` and
/// `before_long_help`, because clap's template has no other place whose
/// text differs between `-h` and `--help`, so a compact group's command
/// loses its own `before_help` and `before_long_help`.
///
/// Each leaf's help ends with its `Examples:` section after the options, in
/// `-h` and in `--help`, as `parse` prints it, and the section replaces the
/// command's own `after_help` and `after_long_help`. `extend` adds no
/// `Columns:` sections and no `Next:` line, because describing the columns
/// builds the row type's JSON Schema, and `extend` runs before parsing and
/// cannot know whether help will print. `extend` adds no `Effects:` line
/// either, because `parse` prints that line with crumbkit's other lines. A
/// leaf without examples leaves its command unchanged, and of two leaves
/// with one path only the first applies, as in `parse`.
///
/// `extend` skips a group whose path names no command or a command without
/// visible subcommands, a leaf whose path names no command, and leaves every
/// command that the tool record does not name with clap's own help.
/// crumbkit-test's `check_help` fails on a tool record that names a missing
/// command or misses a visible one. `extend` reads no topics, because
/// `parse` reads them from the tool record.
///
/// Its work is one formatted line for each subcommand of each group and for
/// each example of each leaf. It never clones the tree.
///
/// [shelf-h]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/help/shelf-h.txt
/// [shelf-help]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/help/shelf-help.txt
#[must_use]
pub fn extend(mut command: Command, tool: &Tool) -> Command {
    // clap copies the root's styles to every subcommand when it builds the
    // tree, so every page with goal headings prints with the root's styles.
    let styles = command.get_styles();
    let header = *styles.get_header();
    let literal = *styles.get_literal();
    let help = tool.get_help();
    let verbs = help.root_verb_names();
    for group in help.groups {
        if let Some(node) = find_mut(&mut command, group.path()) {
            let verbs = if group.path().is_empty() { verbs } else { &[] };
            apply(node, group, verbs, header, literal);
        }
    }
    apply_root_verbs(&mut command, help.groups, verbs, header, literal);
    let tool_name = command.get_name().to_owned();
    for (index, leaf) in help.leaves.iter().enumerate() {
        // `parse` finds the first leaf with a path, so `extend` skips the
        // later ones.
        let earlier = &help.leaves[..index];
        if earlier.iter().any(|other| other.path() == leaf.path()) {
            continue;
        }
        let Some(node) = find_mut(&mut command, leaf.path()) else {
            continue;
        };
        let text = std::iter::once(tool_name.as_str())
            .chain(leaf.path().iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        let short = leaf::examples(leaf, &text, false, header, literal);
        let long = leaf::examples(leaf, &text, true, header, literal);
        if let (Some(short), Some(long)) = (short, long) {
            *node = std::mem::take(node).after_help(short).after_long_help(long);
        }
    }
    command
}

/// Returns the command at `path` below `command`, matching each name against
/// a subcommand's real name, or `None` when a name matches no subcommand.
fn find_mut<'a>(command: &'a mut Command, path: &[&str]) -> Option<&'a mut Command> {
    let mut node = command;
    for &name in path {
        node = node
            .get_subcommands_mut()
            .find(|subcommand| subcommand.get_name() == name)?;
    }
    Some(node)
}
