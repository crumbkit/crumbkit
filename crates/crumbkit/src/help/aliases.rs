//! The guide's function that names a person's aliases on the root's help
//! (ADR-0027).

use clap::Command;

use super::Guide;

impl Guide {
    /// Sets the function that returns the names of a person's aliases, such
    /// as `crumbkit_aliases::names`, so the `-h` and `--help` of the clap
    /// root end with a line such as ``Personal aliases: firsts, mine.
    /// `shelf aliases list` shows what each runs.``, between the `Topics:`
    /// line and the `Next:` line. A person's aliases expand only in the
    /// first word after the tool's name, so the line is on the clap root's
    /// help alone.
    ///
    /// [`parse`](crate::parse) calls `names` with the clap root only when
    /// it prints the root's help, so no other run reads a person's aliases.
    /// The function reads the tool's name from the root, and can leave out
    /// an alias that a real command shadows. When `names` returns no names,
    /// the help has no such line.
    ///
    /// The root's help then differs between machines, so a tool that sets
    /// it opts out of crumbkit-test's `Rule::PersonalAliases`, and
    /// crumbkit-test's help checks print help without the line (ADR-0027).
    #[must_use]
    pub const fn personal_aliases(mut self, names: fn(&Command) -> Vec<String>) -> Self {
        self.personal_aliases = Some(names);
        self
    }

    /// Returns the guide without the function that
    /// [`Guide::personal_aliases`] set, so the root's help is the same on
    /// every machine, as crumbkit-test's help checks print it.
    #[must_use]
    pub const fn without_personal_aliases(mut self) -> Self {
        self.personal_aliases = None;
        self
    }

    /// Returns the function that [`Guide::personal_aliases`] set, or `None`
    /// when help names no aliases. The name has clap's `get_` prefix,
    /// because `personal_aliases` is the builder method.
    #[must_use]
    pub const fn get_personal_aliases(&self) -> Option<fn(&Command) -> Vec<String>> {
        self.personal_aliases
    }
}
