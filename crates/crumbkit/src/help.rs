//! The guide to a tool's help: goal headings, which list a group's
//! subcommands under headings that the tool declares, such as `Read` and
//! `Change`, in place of clap's single `Commands:` list, and help topics,
//! which `<tool> help <TOPIC>` prints.

pub(crate) mod topic;

use std::fmt::Write as _;

use clap::Command;
use clap::builder::StyledStr;
use clap::builder::styling::Style;

pub use self::topic::Topic;

#[cfg(test)]
mod tests;

/// The guide to a tool's help pages: one [`Group`] for each command whose
/// help lists its subcommands under goals, and the tool's [`Topic`]s.
///
/// A tool declares its guide once as a `const` and applies it with
/// [`extend`] before it parses:
///
/// ```
/// use clap::{CommandFactory, Parser, Subcommand};
/// use crumbkit::Spelling;
/// use crumbkit::help::{Goal, Group, Guide, Topic};
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
///     .topics(&[Topic::new(
///         "catalogue",
///         "Where the books come from",
///         "The catalogue is built into the binary.\n",
///     )]);
///
/// let command = crumbkit::help::extend(Cli::command(), &GUIDE);
/// let parsed = crumbkit::parse_from::<Cli, _, _>(command, Spelling::COLONS, ["shelf", "books"]);
/// assert!(parsed.is_ok());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Guide {
    /// The commands whose help lists their subcommands under goals.
    groups: &'static [Group],
    /// The topics that `<tool> help` lists, in order.
    topics: &'static [Topic],
}

impl Guide {
    /// Returns a guide with no groups and no topics, which changes no help
    /// page.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            groups: &[],
            topics: &[],
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

    /// Sets the topics that `<tool> help` lists, in the order it lists them.
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
}

/// A command whose help lists its subcommands under goal headings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// The names of the subcommands from the root to the command.
    path: &'static [&'static str],
    /// The goals, in the order the help page lists them.
    goals: &'static [Goal],
}

impl Group {
    /// Returns the group of the command at `path`, the real names of its
    /// subcommands from the root, as [`Call::new`](crate::Call::new) names a
    /// command. An empty path names the root. Its help lists `goals` in
    /// order.
    #[must_use]
    pub const fn new(path: &'static [&'static str], goals: &'static [Goal]) -> Self {
        Self { path, goals }
    }

    /// Returns the names of the subcommands from the root to the command.
    #[must_use]
    pub const fn path(&self) -> &'static [&'static str] {
        self.path
    }

    /// Returns the goals, in the order the help page lists them.
    #[must_use]
    pub const fn goals(&self) -> &'static [Goal] {
        self.goals
    }
}

/// A heading on a group's help page and the subcommands listed under it,
/// such as `Read` with `list` and `show`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Goal {
    /// The heading, which prints without a colon.
    heading: &'static str,
    /// The real names of the subcommands, in the order the heading lists
    /// them.
    commands: &'static [&'static str],
}

impl Goal {
    /// Returns the goal that lists `commands` under `heading`. `commands`
    /// holds each subcommand's real name, never an alias.
    #[must_use]
    pub const fn new(heading: &'static str, commands: &'static [&'static str]) -> Self {
        Self { heading, commands }
    }

    /// Returns the heading.
    #[must_use]
    pub const fn heading(&self) -> &'static str {
        self.heading
    }

    /// Returns the real names of the subcommands, in the order the heading
    /// lists them.
    #[must_use]
    pub const fn commands(&self) -> &'static [&'static str] {
        self.commands
    }
}

/// Applies `guide` to `command`, the root of the tool's unbuilt tree, and
/// returns the tree.
///
/// A tool's `main` calls it once before [`parse`](crate::parse), because
/// parsing builds the tree. `parse` answers `help` itself and turns off
/// clap's generated `help` subcommand, so the lists never show it.
///
/// Each group's help page lists its visible subcommands under its goals'
/// headings, in the guide's order, and then the visible subcommands that no
/// goal names under a final `Commands` heading:
///
/// ```text
/// Read
///   list    List the planets
///   show    Show planets by name
///
/// Change
///   rename  Rename a planet
/// ```
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
/// `extend` skips a group whose path names no command or a command without
/// visible subcommands, and leaves every command that the guide does not
/// name with clap's own `Commands:` list. crumbkit-test's `check_guide`
/// fails on a guide that names a missing command or misses a visible one.
///
/// When the guide has topics, `extend` adds a hidden root command named
/// `help` that holds them, and turns off clap's generated `help` subcommand
/// so the tree never has two. `parse` reads the topics from that command:
/// `<tool> help` lists them, `<tool> help <TOPIC>` prints one, and the
/// root's help names them. `extend` keeps the first of two topics with one
/// name, and adds no topics when the root already has a subcommand named or
/// aliased `help`.
///
/// Its work is one formatted line for each subcommand of each group, and
/// one command for each topic, which holds the topic's text. It never
/// clones the tree.
#[must_use]
pub fn extend(mut command: Command, guide: &Guide) -> Command {
    // clap copies the root's styles to every subcommand when it builds the
    // tree, so every guided page prints with the root's styles.
    let styles = command.get_styles();
    let header = *styles.get_header();
    let literal = *styles.get_literal();
    let template = template(header);
    for group in guide.groups {
        let Some(node) = find_mut(&mut command, group.path) else {
            continue;
        };
        let Some(list) = list(node, group.goals, header, literal) else {
            continue;
        };
        // `--help` falls back to `before_help` when `before_long_help` is
        // unset, so the list shows in both. A set `before_long_help` would
        // make clap's `-h` line say `(see more with '--help')` on a page
        // whose `--help` shows nothing more.
        //
        // RUST-10: the list replaces a tool's own `before_help` and
        // `before_long_help` on a guided command. A tool that needs its own
        // text there keeps that command out of the guide, until a template
        // tag of crumbkit's own replaces `before_help` as the list's place.
        *node = std::mem::take(node)
            .help_template(template.clone())
            .before_help(list)
            .before_long_help(None::<&str>);
    }
    topic::add(command, guide.topics)
}

/// Returns the help template of a guided command, with `header` as the style
/// of its `Options:` heading.
///
/// It is clap's default template with the guide's list in `{before-help}` in
/// place of `{all-args}`. clap expands no tags in `before_help`, so a `{` in
/// a description prints as it is. The one exception is `{n}`, which clap
/// turns into a line break in `before_help`.
//
// RUST-10: the template copies the layout of clap's default template, and
// shelf's golden files fail when clap changes that layout. It handles
// options only: a guided command shows no `Arguments:` section for its
// positional arguments, and `{options}` lists an argument that has its own
// help heading under `Options:` (clap's help_template.rs). A template that
// writes each of clap's sections replaces it when a guided command needs
// positional arguments or help headings.
fn template(header: Style) -> String {
    format!(
        "{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n{{before-help}}{header}Options:{header:#}\n{{options}}{{after-help}}"
    )
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

/// Returns the headed list of the visible subcommands of `node` under
/// `goals`, with `header` as the style of each heading and `literal` as the
/// style of each name, or `None` when it lists no subcommand.
//
// RUST-10: each line holds the name and the description and nothing else.
// It leaves out visible aliases, which crumbkit-test's `check_names`
// forbids. A description prints without its own styles, because clap
// offers a description's styled text only with its `color` feature. A
// description that clap's `wrap_help` feature wraps, or that
// holds a line break, continues at column 0, where clap's own list indents
// it under the description's first line. A list that indents continuation
// lines as clap does replaces it when a tool needs long descriptions.
fn list(node: &Command, goals: &[Goal], header: Style, literal: Style) -> Option<String> {
    let mut sections: Vec<(&str, Vec<&Command>)> = Vec::new();
    for goal in goals {
        let commands: Vec<&Command> = goal
            .commands
            .iter()
            .filter_map(|&name| {
                node.get_subcommands()
                    .find(|subcommand| subcommand.get_name() == name)
            })
            .filter(|subcommand| !subcommand.is_hide_set())
            .collect();
        if !commands.is_empty() {
            sections.push((goal.heading, commands));
        }
    }
    let unnamed: Vec<&Command> = node
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .filter(|subcommand| {
            !goals
                .iter()
                .any(|goal| goal.commands.contains(&subcommand.get_name()))
        })
        .collect();
    if !unnamed.is_empty() {
        sections.push(("Commands", unnamed));
    }

    let width = sections
        .iter()
        .flat_map(|(_, commands)| commands)
        .map(|command| command.get_name().chars().count())
        .max()?;
    let mut text = String::new();
    for (heading, commands) in sections {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        // Writing to a `String` cannot fail.
        let _ = write!(text, "{header}{heading}{header:#}");
        for command in commands {
            let about = command.get_about().or_else(|| command.get_long_about());
            write_line(&mut text, command.get_name(), about, width, literal);
        }
    }
    Some(text)
}

/// Writes one line of a list to `text`, which the goal list and the topic
/// list share: a line break, two spaces and `name` in the `literal` style,
/// then, when there is an `about`, padding to `width`, the length of the
/// longest name in the list, plus two spaces, and the `about`.
fn write_line(
    text: &mut String,
    name: &str,
    about: Option<&StyledStr>,
    width: usize,
    literal: Style,
) {
    // Writing to a `String` cannot fail.
    let _ = write!(text, "\n  {literal}{name}{literal:#}");
    if let Some(about) = about {
        let padding = width - name.chars().count() + 2;
        let _ = write!(text, "{:padding$}{about}", "");
    }
}
