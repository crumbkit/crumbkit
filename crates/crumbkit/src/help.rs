//! The guide to a tool's help: goal headings, which list a group's
//! subcommands under headings that the tool declares, such as `Read` and
//! `Change`, in place of clap's single `Commands:` list, leaves, which give
//! a command examples and describe its output columns, help topics, which
//! `<tool> help <TOPIC>` prints, and file formats, whose schemas
//! `<tool> schemas show` prints.

mod aliases;
mod dashed;
pub(crate) mod effects;
pub(crate) mod exit;
mod format;
mod group;
pub(crate) mod leaf;
mod list;
mod output;
pub(crate) mod rename;
mod runtime;
mod spelling;
mod terminal;
pub(crate) mod topic;
pub(crate) mod view;
pub(crate) mod words;

use clap::Command;

pub use self::effects::{Effects, Items};
pub use self::exit::Exit;
pub use self::format::FileFormat;
pub use self::group::{Goal, Group};
pub use self::leaf::{
    BYTES_LINE, Column, Example, HEADED_LINE, Leaf, RefusalContext, STREAM_LINE, children_line,
    text_line,
};
pub use self::rename::Rename;
pub use self::runtime::RuntimeLeaf;
pub use self::topic::Topic;
pub use self::view::ViewFlag;
pub use self::words::quote;

pub(crate) use self::group::apply;
use self::list::write_line;
use crate::pick::Picker;
use crate::{JsonLayout, Spelling, ToolName};

#[cfg(test)]
mod tests;

/// The guide to a tool's help pages: one [`Group`] for each command whose
/// help lists its subcommands under goals, one [`Leaf`] for each command
/// whose help shows examples and describes its columns, the tool's
/// [`Topic`]s, the [`FileFormat`]s of the files that users write for the
/// tool, the [`Exit`] codes that the tool returns besides crumbkit's 0, 1
/// and 2, the [`Spelling`] of its failure codes, and the function that
/// names a person's aliases on the root's help.
///
/// A tool declares its guide once as a `const` and passes it to
/// [`parse`](crate::parse), which applies its goal headings to the help of
/// the commands on the typed path, applies the leaf of the deepest command
/// typed, and prints its topics:
///
/// ```
/// use clap::{CommandFactory, Parser, Subcommand};
/// use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};
///
/// /// Browse a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(subcommand)]
///     resource: Resource,
/// }
///
/// #[derive(Subcommand)]
/// enum Resource {
///     /// Work with the books in the catalogue.
///     Books,
/// }
///
/// const GUIDE: Guide = Guide::new()
///     .groups(&[Group::new(
///         &[],
///         &[Goal::new("Browse the catalogue", &["books"])],
///     )])
///     .leaves(&[Leaf::new(&["books"]).examples(&[Example::new(&[], "List the books")])])
///     .topics(&[Topic::new(
///         "catalogue",
///         "Where the books come from",
///         "The catalogue is built into the binary.\n",
///     )]);
///
/// let args = ["shelf", "books"];
/// let parsed = crumbkit::parse_from::<Cli, _, _>(Cli::command(), &GUIDE, args);
/// assert!(parsed.is_ok());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Guide {
    /// The commands whose help lists their subcommands under goals.
    groups: &'static [Group],
    /// The commands whose help shows examples and describes their columns.
    leaves: &'static [Leaf],
    /// The topics that `<tool> help <TOPIC>` prints and that the root's
    /// help names, in order.
    topics: &'static [Topic],
    /// The file formats that the schema commands serve, in order.
    documents: &'static [FileFormat],
    /// The tool's own exit codes, which `<tool> help exit-codes` lists.
    exits: &'static [Exit],
    /// How the tool spells its failure codes, which [`Guide::spelling`]
    /// sets.
    spelling: Spelling,
    /// The tool's own codes for crumbkit's failures.
    codes: &'static [Rename],
    /// The function that builds refusal alternatives for a command whose
    /// leaf sets none, or `None` for crumbkit's alternatives alone.
    refusal_alternatives: Option<fn(&RefusalContext<'_>) -> Vec<crate::Call>>,
    /// The function that returns the names of a person's aliases, which
    /// the root's help names, or `None` when help names no aliases.
    personal_aliases: Option<fn(&Command) -> Vec<String>>,
    /// The picker that asks a person for a missing argument, or `None` when
    /// `parse` refuses every command line that leaves one out.
    picker: Option<&'static dyn Picker>,
    /// The reason the tool gives for turning terminal detection off, or
    /// `None` when crumbkit asks whether a stream is a terminal (ADR-0049).
    without_terminal_detection: Option<&'static str>,
    /// The most bytes that one run prints on standard output, with the
    /// reason the tool gives for it, or `None` for crumbkit's default.
    output_budget: Option<(usize, &'static str)>,
    /// The layout of a whole JSON document, which
    /// [`Guide::json_layout`] sets (ADR-0052).
    json_layout: JsonLayout,
    /// The path from the clap root to the command that callers run first,
    /// which answers `help` with the topics.
    root: &'static [&'static str],
    /// Whether a command named in another style answers to its dashed form.
    dashed_forms: bool,
}

impl Guide {
    /// Returns a guide with no groups, no leaves, no topics, no file
    /// formats and no exit codes of the tool's own.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            groups: &[],
            leaves: &[],
            topics: &[],
            documents: &[],
            exits: &[],
            spelling: Spelling::COLONS,
            codes: &[],
            refusal_alternatives: None,
            personal_aliases: None,
            picker: None,
            without_terminal_detection: None,
            output_budget: None,
            json_layout: JsonLayout::OneLine,
            root: &[],
            dashed_forms: false,
        }
    }

    /// Sets the commands whose help lists their subcommands under goals.
    #[must_use]
    pub const fn groups(mut self, groups: &'static [Group]) -> Self {
        self.groups = groups;
        self
    }

    /// Returns the groups that [`Guide::groups`] set. The name has clap's
    /// `get_` prefix, because `groups` is the builder method.
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

    /// Returns the leaves that [`Guide::leaves`] set. The name has clap's
    /// `get_` prefix, because `leaves` is the builder method.
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

    /// Returns the topics that [`Guide::topics`] set. The name has clap's
    /// `get_` prefix, because `topics` is the builder method.
    #[must_use]
    pub const fn get_topics(&self) -> &'static [Topic] {
        self.topics
    }

    /// Sets the file formats that users write and no command prints, such
    /// as a configuration file, which `schemas list` and `schemas show`
    /// serve after the documents of the leaves, in this order (ADR-0035).
    #[must_use]
    pub const fn documents(mut self, documents: &'static [FileFormat]) -> Self {
        self.documents = documents;
        self
    }

    /// Returns the file formats that [`Guide::documents`] set, which the
    /// schema commands serve. The name has clap's `get_` prefix, because
    /// `documents` is the builder method.
    pub(crate) const fn get_documents(&self) -> &'static [FileFormat] {
        self.documents
    }

    /// Sets the exit codes that the tool returns besides crumbkit's 0, 1
    /// and 2, in the order that `<tool> help exit-codes` lists them. An
    /// exit can list the tool's failure codes and crumbkit's, so a tool
    /// gives a refused command line its own number, such as 64, by listing
    /// `crumbkit::codes::CLI_USAGE` or the tool's rename of it (ADR-0056).
    #[must_use]
    pub const fn exits(mut self, exits: &'static [Exit]) -> Self {
        self.exits = exits;
        self
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

    /// Returns the path that [`Guide::root`] set, or an empty path for the
    /// clap root. The name has clap's `get_` prefix, because `root` is the
    /// builder method.
    #[must_use]
    pub const fn get_root(&self) -> &'static [&'static str] {
        self.root
    }

    /// Returns the path that [`Guide::root`] set when it names a command of
    /// `command`, the tool's clap root, and otherwise the empty path.
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

    /// Returns the name that the output of `command`, the tool's clap root,
    /// gives crumbkit's own codes, documents and variables (ADR-0079). It
    /// is the root command's name, or the last word of the path that
    /// [`Guide::root`] sets when that path names a command, so a cargo
    /// subcommand that sets `&["crumbkit"]` is named `crumbkit`.
    ///
    /// crumbkit-test's checks read the tool's names through it.
    ///
    /// ```
    /// use clap::Command;
    /// use crumbkit::help::Guide;
    ///
    /// let tool = Guide::new().tool_name(&Command::new("my-tool"));
    /// assert_eq!(tool.schema_id("crumbkit/failure/v1"), "my-tool/failure/v1");
    /// ```
    #[must_use]
    pub fn tool_name(&self, command: &Command) -> ToolName {
        ToolName::new(
            self.root_in(command)
                .last()
                .copied()
                .unwrap_or_else(|| command.get_name()),
        )
    }

    /// Sets the picker that asks a person for the value of a missing
    /// required argument, such as [`Menu`](crate::pick::Menu) (ADR-0043).
    ///
    /// When clap refuses a command line because it leaves out a required
    /// argument of a leaf, [`parse`](crate::parse) asks the picker to pick
    /// the argument's values from its possible values, or else from the
    /// source that the leaf's [`Leaf::choices`] names. It asks only when a
    /// person can answer: standard input and standard error are terminals,
    /// no coding agent runs the tool, the caller did not pass `--no-input`,
    /// and the guide keeps terminal detection on
    /// ([`Guide::without_terminal_detection`]). It then adds the picked
    /// values to the command line, prints the whole command line on
    /// standard error after `Running:`, and parses again. Without a picker, or when it cannot ask, `parse`
    /// refuses the command line as clap does.
    #[must_use]
    pub const fn picker(mut self, picker: &'static dyn Picker) -> Self {
        self.picker = Some(picker);
        self
    }

    /// Returns the picker that [`Guide::picker`] set, or `None`. The name
    /// has clap's `get_` prefix, because `picker` is the builder method.
    #[must_use]
    pub const fn get_picker(&self) -> Option<&'static dyn Picker> {
        self.picker
    }

    /// Returns the exit codes that [`Guide::exits`] set. The name has
    /// clap's `get_` prefix, because `exits` is the builder method.
    #[must_use]
    pub const fn get_exits(&self) -> &'static [Exit] {
        self.exits
    }
}

impl Default for Guide {
    /// Returns [`Guide::new`]. `Spelling` has no `Default`, so the guide
    /// cannot derive it.
    fn default() -> Self {
        Self::new()
    }
}

/// Applies the goal headings and the leaves' examples of `guide` to the
/// whole tree of `command`, the root of the tool's unbuilt tree, and
/// returns the tree.
///
/// [`parse`](crate::parse) takes the guide and applies its goal headings to
/// the groups that the typed words pass through, and the leaf of the
/// deepest command typed, so a tool that parses with crumbkit does not call
/// `extend`. A tool that parses with plain clap calls it once before
/// parsing, and a test calls it to check the whole guided tree, such as
/// with clap's `Command::debug_assert`.
///
/// Each group's help page lists its visible subcommands under its goals'
/// headings, in the guide's order, and then the visible subcommands that no
/// goal names under a final `Commands` heading. shelf's root group is
/// compact: [`shelf-h.txt`][shelf-h] shows its `-h` and
/// [`shelf-help.txt`][shelf-help] its `--help`.
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
/// The guided command's own `before_help` prints at the top of `-h`, and its
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
/// command that the guide does not name with clap's own help.
/// crumbkit-test's `check_guide` fails on a guide that names a missing
/// command or misses a visible one. `extend` reads no topics, because
/// `parse` reads them from the guide.
///
/// Its work is one formatted line for each subcommand of each group and for
/// each example of each leaf. It never clones the tree.
///
/// [shelf-h]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/help/shelf-h.txt
/// [shelf-help]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/help/shelf-help.txt
#[must_use]
pub fn extend(mut command: Command, guide: &Guide) -> Command {
    // clap copies the root's styles to every subcommand when it builds the
    // tree, so every guided page prints with the root's styles.
    let styles = command.get_styles();
    let header = *styles.get_header();
    let literal = *styles.get_literal();
    for group in guide.groups {
        if let Some(node) = find_mut(&mut command, group.path()) {
            apply(node, group, header, literal);
        }
    }
    let tool = command.get_name().to_owned();
    for (index, leaf) in guide.leaves.iter().enumerate() {
        // `parse` finds the first leaf with a path, so `extend` skips the
        // later ones.
        let earlier = &guide.leaves[..index];
        if earlier.iter().any(|other| other.path() == leaf.path()) {
            continue;
        }
        let Some(node) = find_mut(&mut command, leaf.path()) else {
            continue;
        };
        let text = std::iter::once(tool.as_str())
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
