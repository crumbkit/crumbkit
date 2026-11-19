//! Help topics, which [`parse`](crate::parse) reads from the tool record it
//! takes through [`text`] and [`names`]. Each of them adds crumbkit's own
//! `exit-codes` topic after the tool's topics.

use std::borrow::Cow;

use super::exit;
use crate::Tool;

mod step;
pub use self::step::Step;

#[cfg(test)]
mod tests;

/// A help topic: text that only the tool can write, such as where its data
/// comes from, which `<tool> help <TOPIC>` prints.
///
/// The text is plain text, written as a literal or with `include_str!`.
/// crumbkit prints it as written and adds a final line break when it lacks
/// one. A tool lists its topics in its [`Tool`](super::Tool):
///
/// ```
/// use crumbkit::Tool;
/// use crumbkit::help::{Help, Topic};
///
/// const TOOL: Tool = Tool::new().help(Help::new().topics(&[Topic::new(
///     "catalogue",
///     "Where the books come from",
///     "The catalogue holds six books, built into the binary.\n",
/// )]));
///
/// assert_eq!(TOOL.get_help().get_topics()[0].name(), "catalogue");
/// ```
///
/// # Guides
///
/// [`Topic::guide`] declares a task guide in place of text: a list of
/// [`Step`]s, each with a one-line explanation and the commands that carry
/// it out. crumbkit prints the steps as a numbered list, and crumbkit-test's
/// `check_help` parses every command against the tool's clap tree, so a guide
/// stops passing when the tool renames a flag that a step uses. A guide has at
/// most [`Topic::MAX_STEPS`] steps and a step at most [`Step::MAX_COMMANDS`]
/// commands, and `check_help` fails a guide that has more. [`Step::see`]
/// declares a step that another tool's topic carries out.
///
/// [`Topic::text`] returns an empty string for a guide, because the printed
/// text holds the tool's name. A tool that prints or lists topics itself
/// calls [`Topic::render`] with its name, which works for both kinds.
///
/// ```
/// use crumbkit::Tool;
/// use crumbkit::help::{Help, Step, Topic};
///
/// const TOOL: Tool = Tool::new().help(Help::new().topics(&[Topic::guide(
///     "first-record",
///     "Sign in and create a record",
///     &[
///         Step::new("Sign in.", &[&["login", "--user", "you@example.com"]]),
///         Step::new("Create a record.", &[&["create", "--title", "Quarterly report"]]),
///     ],
/// )]));
///
/// let text = TOOL.get_help().get_topics()[0].render("records");
/// assert_eq!(
///     text,
///     "1. Sign in.\n   $ records login --user you@example.com\n\
///      2. Create a record.\n   $ records create --title 'Quarterly report'\n"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topic {
    /// The word after `help` that prints the topic.
    name: &'static str,
    /// The summary of the topic, which the tool reads through
    /// [`Topic::about`].
    about: &'static str,
    /// The text that `<tool> help <TOPIC>` prints, empty for a guide.
    text: &'static str,
    /// The steps of a guide, or `None` for a topic of plain text.
    steps: Option<&'static [Step]>,
}

impl Topic {
    /// Returns the topic called `name` with the summary `about`, which
    /// `<tool> help <name>` prints as `text`. crumbkit prints `about`
    /// nowhere, because `<tool> help` prints the root's help, whose
    /// `Topics:` line names each topic (ADR-0101).
    #[must_use]
    pub const fn new(name: &'static str, about: &'static str, text: &'static str) -> Self {
        Self {
            name,
            about,
            text,
            steps: None,
        }
    }

    /// The most steps that a guide has. `check_help` fails a guide with more,
    /// and the tool splits it into two guides.
    pub const MAX_STEPS: usize = 12;

    /// Returns the guide called `name` with the summary `about`, which
    /// `<tool> help <name>` prints as one numbered line for each of `steps`
    /// and the step's commands under it. `about` is used as for
    /// [`Topic::new`]. A guide has between one and [`Topic::MAX_STEPS`]
    /// steps.
    #[must_use]
    pub const fn guide(name: &'static str, about: &'static str, steps: &'static [Step]) -> Self {
        Self {
            name,
            about,
            text: "",
            steps: Some(steps),
        }
    }

    /// Returns the word after `help` that prints the topic.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the summary that the topic was declared with. crumbkit
    /// prints it nowhere, and a tool that lists its topics itself reads it
    /// here.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
    }

    /// Returns the text that a [`Topic::new`] topic was declared with, as
    /// written, and an empty string for a [guide](Topic::guide), whose text
    /// holds the tool's name and which [`Topic::render`] prints.
    #[must_use]
    pub const fn text(&self) -> &'static str {
        self.text
    }

    /// Returns the steps of a [guide](Topic::guide), or `None` for a topic
    /// of plain text.
    #[must_use]
    pub const fn steps(&self) -> Option<&'static [Step]> {
        self.steps
    }

    /// Returns what `<tool_name> help <TOPIC>` prints: the text as written
    /// for a plain topic, and for a guide the numbered steps with each
    /// command as `$` and the tool's name and the command's words quoted as
    /// [`quote`](super::quote) does. A guide's text ends with a line break,
    /// and a plain topic's text ends as it was written.
    #[must_use]
    pub fn render(&self, tool_name: &str) -> Cow<'static, str> {
        match self.steps {
            None => Cow::Borrowed(self.text),
            Some(steps) => Cow::Owned(step::render(tool_name, steps)),
        }
    }
}

/// Returns the topics in declared order, leaving out each topic whose name an
/// earlier topic already has, so the first of two topics with one name is
/// the one that prints, and a topic named `exit-codes`, because crumbkit's
/// own topic has that name.
fn distinct(topics: &[Topic]) -> impl Iterator<Item = &Topic> {
    topics.iter().enumerate().filter_map(|(index, topic)| {
        let repeated = topics[..index]
            .iter()
            .any(|earlier| earlier.name == topic.name);
        (!repeated && topic.name != exit::TOPIC).then_some(topic)
    })
}

/// Returns the text of the topic called `name` in `tool` of the tool called
/// `tool`, ending with a line break, or `None` when the tool record has no such
/// topic. The name `exit-codes` gives crumbkit's list of exit codes.
pub(crate) fn text(tool_name: &str, tool: &Tool, name: &str) -> Option<String> {
    if name == exit::TOPIC {
        return Some(exit::text(tool_name, tool));
    }
    let topic = tool
        .get_help()
        .get_topics()
        .iter()
        .find(|topic| topic.name == name)?;
    let mut text = topic.render(tool_name).into_owned();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// Returns the names of `topics` in declared order, each once, and then
/// `exit-codes`.
pub(crate) fn names(topics: &[Topic]) -> Vec<&str> {
    distinct(topics)
        .map(|topic| topic.name)
        .chain(std::iter::once(exit::TOPIC))
        .collect()
}
