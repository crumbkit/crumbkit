//! Help topics, which [`parse`](crate::parse) reads from the guide it takes
//! through [`text`], [`list`] and [`names`]. Each of them adds crumbkit's
//! own `exit-codes` topic after the tool's topics.

use clap::builder::StyledStr;
use clap::builder::styling::Style;

use super::{Guide, exit, write_line};

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

/// Returns the topics in guide order, leaving out each topic whose name an
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

/// Returns the text of the topic called `name` in `guide` of the tool
/// called `tool`, ending with a line break, or `None` when the guide has no
/// such topic. The name `exit-codes` gives crumbkit's list of exit codes.
pub(crate) fn text(tool: &str, guide: &Guide, name: &str) -> Option<String> {
    if name == exit::TOPIC {
        return Some(exit::text(tool, guide));
    }
    let topic = guide.get_topics().iter().find(|topic| topic.name == name)?;
    let mut text = topic.text.to_owned();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// Returns the page that `<tool> help` prints, where `tool` is the tool's
/// name: the topics in guide order and then `exit-codes` under the heading
/// `Topics`, in the layout of the goal list, and a `Next:` line that prints
/// one.
pub(crate) fn list(tool: &str, topics: &[Topic]) -> String {
    let width = distinct(topics)
        .map(|topic| topic.name.chars().count())
        .chain(std::iter::once(exit::TOPIC.len()))
        .max()
        .unwrap_or_default();
    let mut text = String::from("Topics");
    for topic in distinct(topics) {
        let about = (!topic.about.is_empty()).then(|| StyledStr::from(topic.about));
        write_line(&mut text, topic.name, about.as_ref(), width, Style::new());
    }
    let about = StyledStr::from(exit::ABOUT);
    write_line(&mut text, exit::TOPIC, Some(&about), width, Style::new());
    text.push_str("\n\nNext: ");
    text.push_str(tool);
    text.push_str(" help <TOPIC>\n");
    text
}

/// Returns the names of `topics` in guide order, each once, and then
/// `exit-codes`.
pub(crate) fn names(topics: &[Topic]) -> Vec<&str> {
    distinct(topics)
        .map(|topic| topic.name)
        .chain(std::iter::once(exit::TOPIC))
        .collect()
}
