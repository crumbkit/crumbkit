//! Help topics, and the hidden `help` command that carries them from
//! [`extend`](super::extend) to [`parse`](crate::parse).
//!
//! `parse` takes the command tree and no guide, so `extend` keeps the
//! guide's topics in the tree: a hidden root command named `help`, whose
//! subcommand value name is `TOPIC`, with one subcommand for each topic. A
//! topic's subcommand holds the topic's name as its name, the topic's
//! `about` as its `about` and the topic's text as its `long_about`. `parse`
//! reads them back through [`text`], [`list`] and [`names`]. This module is
//! the only code that writes or reads that layout.

use clap::Command;
use clap::builder::styling::Style;

use super::write_line;

#[cfg(test)]
mod tests;

/// A help topic: text that only the tool can write, such as where its data
/// comes from, which `<tool> help <TOPIC>` prints.
///
/// The text is plain text, written as a literal or with `include_str!`.
/// crumbkit prints it as written and adds a final line break when it lacks
/// one. A tool lists its topics in its [`Guide`](super::Guide):
///
/// ```
/// use crumbkit::help::{Guide, Topic};
///
/// const GUIDE: Guide = Guide::new().topics(&[Topic::new(
///     "catalogue",
///     "Where the books come from",
///     "The catalogue holds six books, built into the binary.\n",
/// )]);
///
/// assert_eq!(GUIDE.get_topics()[0].name(), "catalogue");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topic {
    /// The word after `help` that prints the topic.
    name: &'static str,
    /// The summary that `<tool> help` prints beside the name.
    about: &'static str,
    /// The text that `<tool> help <TOPIC>` prints.
    text: &'static str,
}

impl Topic {
    /// Returns the topic called `name`, which `<tool> help` lists with the
    /// summary `about`, and which `<tool> help <name>` prints as `text`. A
    /// topic with an empty `about` is listed by its name alone.
    #[must_use]
    pub const fn new(name: &'static str, about: &'static str, text: &'static str) -> Self {
        Self { name, about, text }
    }

    /// Returns the word after `help` that prints the topic.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the summary that `<tool> help` prints beside the name.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
    }

    /// Returns the text that `<tool> help <TOPIC>` prints.
    #[must_use]
    pub const fn text(&self) -> &'static str {
        self.text
    }
}

/// The name of the hidden command, which is the word that `parse` answers.
const HELP: &str = "help";

/// The subcommand value name of the hidden command, which tells it apart
/// from a tool's own hidden command named `help`.
const TOPIC: &str = "TOPIC";

/// Returns `root` with a hidden `help` command that holds `topics`, and with
/// clap's generated `help` subcommand turned off, so `Command::build` and
/// `Command::debug_assert` never meet two commands named `help`.
///
/// It returns `root` unchanged when `topics` is empty, or when `root`
/// already has a subcommand named or aliased `help`, which crumbkit-test's
/// `check_grammar` refuses. It keeps the first of two topics with one name.
//
// RUST-10: clap has no stable way to attach a tool's own data to a
// `Command`. `Command::add` and `Command::get` need clap's `unstable-ext`
// feature, so the topics travel as a hidden command. A stable extension
// slot in clap, or a `parse` that takes the guide, replaces the hidden
// command.
pub(super) fn add(root: Command, topics: &[Topic]) -> Command {
    if topics.is_empty() || root.find_subcommand(HELP).is_some() {
        return root;
    }
    let mut help = Command::new(HELP).hide(true).subcommand_value_name(TOPIC);
    for (index, topic) in topics.iter().enumerate() {
        if topics[..index]
            .iter()
            .any(|earlier| earlier.name == topic.name)
        {
            continue;
        }
        let mut page = Command::new(topic.name).long_about(topic.text);
        if !topic.about.is_empty() {
            page = page.about(topic.about);
        }
        help = help.subcommand(page);
    }
    root.disable_help_subcommand(true).subcommand(help)
}

/// Returns the hidden command that holds the topics of `root`, or `None`
/// when the tool has no topics.
fn find(root: &Command) -> Option<&Command> {
    root.get_subcommands().find(|command| holds_topics(command))
}

/// Returns whether `command` is the hidden `help` command that [`add`]
/// gives the root to hold the topics, which `parse` answers in place of
/// clap.
pub(crate) fn holds_topics(command: &Command) -> bool {
    command.get_name() == HELP
        && command.is_hide_set()
        && command.get_subcommand_value_name() == Some(TOPIC)
}

/// Returns the text of the topic called `name`, ending with a line break,
/// or `None` when `root` has no such topic.
pub(crate) fn text(root: &Command, name: &str) -> Option<String> {
    let page = find(root)?
        .get_subcommands()
        .find(|page| page.get_name() == name)?;
    let mut text = page.get_long_about()?.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// Returns the page that `<tool> help` prints for `root`: its topics in
/// guide order under the heading `Topics`, in the layout of the goal list,
/// and a `Next:` line that prints one. It returns `None` when the tool has
/// no topics.
pub(crate) fn list(root: &Command) -> Option<String> {
    let help = find(root)?;
    let width = help
        .get_subcommands()
        .map(|page| page.get_name().chars().count())
        .max()?;
    let mut text = String::from("Topics");
    for page in help.get_subcommands() {
        write_line(
            &mut text,
            page.get_name(),
            page.get_about(),
            width,
            Style::new(),
        );
    }
    text.push_str("\n\nNext: ");
    text.push_str(root.get_name());
    text.push_str(" help <TOPIC>\n");
    Some(text)
}

/// Returns the names of the topics of `root`, in guide order, which is
/// empty when the tool has none.
pub(crate) fn names(root: &Command) -> Vec<&str> {
    find(root)
        .map(|help| help.get_subcommands().map(Command::get_name).collect())
        .unwrap_or_default()
}
