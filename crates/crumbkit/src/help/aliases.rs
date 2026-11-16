//! The function in [`Help`] that names the aliases on the root's help
//! (ADR-0027, ADR-0228).

use clap::Command;

use super::Help;

/// Which aliases the function of [`Help::aliases`] returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// Only the aliases that ship with the tool, which are the same on every
    /// machine. This is the default, and crumbkit-test prints help with this
    /// scope, so golden files match on every machine.
    #[default]
    Shipped,
    /// Every alias, including the ones that differ between machines, such as
    /// the entries of a person's aliases file. A tool that sets this scope
    /// opts out of crumbkit-test's `Rule::PersonalAliases`.
    All,
}

impl Help {
    /// Sets the function that returns the names of the aliases, such as one
    /// that calls `crumbkit_aliases::Aliases::names` for [`Scope::All`] and
    /// `crumbkit_aliases::Aliases::shipped_names` for [`Scope::Shipped`], so
    /// the `-h` and `--help` of the clap root end with a line such as
    /// ``Aliases: firsts, mine. `shelf aliases list` shows what each runs.``,
    /// between the `Topics:` line and the `Next:` line. Aliases expand at the
    /// start of the words after the tool's name, so the line is on the clap
    /// root's help alone.
    ///
    /// [`parse`](crate::parse) calls `names` with the clap root and the
    /// scope of [`Help::alias_scope`] only when it prints the root's help, so
    /// no other run reads an alias source. The function reads the tool's name
    /// from the root, and can leave out an alias that a real command shadows.
    /// When `names` returns no names, the help has no such line.
    ///
    /// The root's help names only shipped aliases unless the tool sets
    /// [`Scope::All`] with [`Help::alias_scope`]. Then the help differs
    /// between machines, so the tool opts out of crumbkit-test's
    /// `Rule::PersonalAliases`, and crumbkit-test's help checks print help
    /// with [`Scope::Shipped`] (ADR-0027, ADR-0228).
    #[must_use]
    pub const fn aliases(mut self, names: fn(&Command, Scope) -> Vec<String>) -> Self {
        self.aliases = Some(names);
        self
    }

    /// Sets the scope that [`parse`](crate::parse) passes to the function of
    /// [`Help::aliases`]. It is [`Scope::Shipped`] unless set. crumbkit-test sets
    /// [`Scope::Shipped`] too, so the root's help is the same on every
    /// machine.
    #[must_use]
    pub const fn alias_scope(mut self, scope: Scope) -> Self {
        self.alias_scope = scope;
        self
    }

    /// Returns the function that [`Help::aliases`] set, or `None` when help
    /// names no aliases.
    #[must_use]
    pub const fn get_aliases(&self) -> Option<fn(&Command, Scope) -> Vec<String>> {
        self.aliases
    }

    /// Returns the scope that [`Help::alias_scope`] set.
    #[must_use]
    pub const fn get_alias_scope(&self) -> Scope {
        self.alias_scope
    }
}
