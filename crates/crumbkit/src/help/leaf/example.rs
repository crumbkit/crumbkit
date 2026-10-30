//! The examples of a leaf, and the `Examples:` section of its help.

use std::fmt::Write as _;

use clap::builder::styling::Style;

use super::Leaf;
use crate::help::words::quote;

/// One example of a leaf: the words after the leaf's path, a one-line
/// description, such as `--all` and `Print every book`, and the names of
/// the preconditions that a run of the example needs, such as `docker`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Example {
    /// The words after the leaf's path, one argument each.
    words: &'static [&'static str],
    /// The one-line description that `--help` prints below the command line.
    about: &'static str,
    /// The names of the preconditions that a run of the example needs.
    needs: &'static [&'static str],
}

impl Example {
    /// Returns the example that runs the leaf with `words`, the arguments
    /// after its path, described by `about`. An empty `words` runs the leaf
    /// without arguments, and an empty `about` prints the command line
    /// alone.
    #[must_use]
    pub const fn new(words: &'static [&'static str], about: &'static str) -> Self {
        Self {
            words,
            about,
            needs: &[],
        }
    }

    /// Sets the names of the preconditions that a run of the example
    /// needs, such as `&["docker"]` for an example that acts on running
    /// containers. A precondition is a name alone, which the tool's tests
    /// match against the names that a test run provides: crumbkit-test's
    /// `check_examples` runs the example only when the run provides every
    /// name, and skips it otherwise. Help and `commands list` do not show
    /// the names.
    #[must_use]
    pub const fn needs(mut self, needs: &'static [&'static str]) -> Self {
        self.needs = needs;
        self
    }

    /// Returns the words after the leaf's path.
    #[must_use]
    pub const fn words(&self) -> &'static [&'static str] {
        self.words
    }

    /// Returns the one-line description.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
    }

    /// Returns the names of the preconditions that a run of the example
    /// needs, which are empty unless [`Example::needs`] set them.
    #[must_use]
    pub const fn get_needs(&self) -> &'static [&'static str] {
        self.needs
    }
}

/// Returns the `Examples:` section of the help of `leaf`, whose command
/// line starts with `text`, the tool's name and the leaf's path, or `None`
/// when the leaf has no examples. `header` styles the heading and `literal`
/// each command line.
///
/// `-h` lists one command line on each line. `--help`, when `long` is set,
/// follows clap's layout of long options: each command line, its
/// description on the next line at clap's indent of 10 spaces, and a blank
/// line between examples.
pub(crate) fn examples(
    leaf: &Leaf,
    text: &str,
    long: bool,
    header: Style,
    literal: Style,
) -> Option<String> {
    let examples = leaf.get_examples();
    if examples.is_empty() {
        return None;
    }
    let mut section = format!("{header}Examples:{header:#}");
    for (index, example) in examples.iter().enumerate() {
        if long && index > 0 {
            section.push('\n');
        }
        // Writing to a `String` cannot fail.
        let _ = write!(section, "\n  {literal}{text}");
        for word in example.words {
            section.push(' ');
            section.push_str(&quote(word));
        }
        let _ = write!(section, "{literal:#}");
        if long && !example.about.is_empty() {
            let _ = write!(section, "\n          {}", example.about);
        }
    }
    Some(section)
}
