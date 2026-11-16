//! The names of the aliases that the root's help prints, for a tool that
//! opts in to showing them (ADR-0027).

use clap::Command;

use crate::Aliases;
use crate::entry::Item;
use crate::source::{Kind, Loaded, Source};

#[cfg(test)]
mod tests;

impl Aliases {
    /// Returns the names of the entries of the listed sources that expand
    /// against `command`, the tool's clap root, in name order and without
    /// repeats, for the tool to pass to `Help::personal_aliases` through a
    /// function of its own:
    ///
    /// ```
    /// use std::sync::LazyLock;
    ///
    /// use clap::Command;
    /// use crumbkit::Tool;
    /// use crumbkit::help::Help;
    /// use crumbkit_aliases::{Aliases, Source};
    ///
    /// static ALIASES: LazyLock<Aliases> = LazyLock::new(|| {
    ///     Aliases::new().source(Source::file(crumbkit_aliases::file("shelf")))
    /// });
    ///
    /// fn alias_names(command: &Command) -> Vec<String> {
    ///     ALIASES.names(command)
    /// }
    ///
    /// const TOOL: Tool = Tool::new().help(Help::new().personal_aliases(alias_names));
    /// ```
    ///
    /// A source added with [`Source::unlisted`] has no names. An entry that
    /// a real command shadows, and an entry whose value is not a list of one
    /// or more words, does not expand, so it has no name here, and
    /// `<tool> aliases list` reports its problem. A missing file, a source
    /// that cannot be read or parsed, and an environment without a home
    /// directory give no names. Like [`expand`](Aliases::expand), it never
    /// prints and never fails.
    ///
    /// crumbkit's `parse` calls the tool's function only when it prints the
    /// root's help, so no other run reads a source.
    ///
    /// [`Source::unlisted`]: crate::Source::unlisted
    #[must_use]
    pub fn names(&self, command: &Command) -> Vec<String> {
        self.collect(command, |_| true)
    }

    /// Returns the names that [`names`](Aliases::names) returns from the
    /// sources of kind [`Kind::Shipped`], which are the same on every
    /// machine, so a help page that names only these is the same on every
    /// machine too.
    #[must_use]
    pub fn shipped_names(&self, command: &Command) -> Vec<String> {
        self.collect(command, |source| source.kind() == Kind::Shipped)
    }

    /// Returns the names of the listed sources that `keep` accepts, sorted
    /// and without repeats.
    fn collect(&self, command: &Command, keep: impl Fn(&Source) -> bool) -> Vec<String> {
        let mut names = Vec::new();
        for source in self
            .sources()
            .iter()
            .filter(|source| source.listed() && keep(source))
        {
            let Loaded::Items(items) = source.load() else {
                continue;
            };
            names.extend(
                items
                    .iter()
                    .filter(|item| item.problem(command, source.reads()).is_none())
                    .map(Item::name),
            );
        }
        // A `DeTable` keeps file order when some crate in the build enables
        // toml's `preserve_order` feature, so the names sort here, as the
        // rows of `aliases list` do.
        names.sort();
        names.dedup();
        names
    }
}
