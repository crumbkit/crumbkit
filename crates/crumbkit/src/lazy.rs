//! `Lazy`, the field type of a command group that builds its subcommands
//! only when the caller types the group's word (ADR-0247).

use clap::{ArgMatches, Command, Error, FromArgMatches, Subcommand};

#[cfg(test)]
mod tests;

/// The value name that clap prints for a group's subcommands when the tool
/// sets none. [`Lazy`] sets it, so help prints the same as without
/// `Lazy`, and [`reach`](crate::reach) recognises a group that is not yet
/// expanded by it.
pub(crate) const MARKER: &str = "COMMAND";

/// A command group whose subcommands and their arguments are built only when
/// the caller types the group's word.
///
/// Declare a group's field as `Lazy<S>` where it would be `S`, with
/// `#[command(subcommand)]` as before, and read its value with
/// [`into_inner`](Self::into_inner) or [`get`](Self::get). `Cli::command()`
/// then builds nothing for `S`, so a run that types another command never
/// pays for the group, and [`parse`](crate::parse()) expands the group when
/// the typed words reach it.
///
/// A group that was not expanded has no subcommands. Code that reads
/// `Cli::command()` directly, such as a test or a tool's own listing, sees
/// the group empty until it calls [`reach`](crate::reach) with the words
/// that name the group, or `Command::build`, which expands every group.
/// [`Run::command`](crate::Run::command) builds the whole tree.
///
/// `Lazy` sets the group's subcommand value name to clap's default,
/// `COMMAND`, when the tool set none, so help reads the same. A tool that
/// sets its own value name with a `#[command(subcommand_value_name)]` after
/// the field, or in `S`, keeps it, and `reach` then reads the group as
/// expanded only when it has subcommands.
///
/// ```
/// use clap::{Parser, Subcommand};
/// use crumbkit::Lazy;
///
/// #[derive(Parser)]
/// struct Cli {
///     #[command(subcommand)]
///     command: Resource,
/// }
///
/// #[derive(Subcommand)]
/// enum Resource {
///     /// Work with skills.
///     Skills {
///         #[command(subcommand)]
///         command: Lazy<SkillsCommand>,
///     },
/// }
///
/// #[derive(Subcommand)]
/// enum SkillsCommand {
///     /// Install the skill.
///     Install,
/// }
///
/// let cli = Cli::try_parse_from(["tool", "skills", "install"]);
/// // `try_parse_from` builds the whole tree, so the group is expanded.
/// let Resource::Skills { command } = cli.unwrap().command;
/// assert!(matches!(command.into_inner(), SkillsCommand::Install));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lazy<S>(S);

impl<S> Lazy<S> {
    /// Returns the group's value.
    #[must_use]
    pub fn into_inner(self) -> S {
        self.0
    }

    /// Returns a reference to the group's value.
    #[must_use]
    pub const fn get(&self) -> &S {
        &self.0
    }
}

impl<S: FromArgMatches> FromArgMatches for Lazy<S> {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        S::from_arg_matches(matches).map(Self)
    }

    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, Error> {
        S::from_arg_matches_mut(matches).map(Self)
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
        self.0.update_from_arg_matches(matches)
    }

    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), Error> {
        self.0.update_from_arg_matches_mut(matches)
    }
}

impl<S: Subcommand> Subcommand for Lazy<S> {
    fn augment_subcommands(command: Command) -> Command {
        marked(command).defer(|command| expand(command, S::augment_subcommands))
    }

    fn augment_subcommands_for_update(command: Command) -> Command {
        marked(command).defer(|command| expand(command, S::augment_subcommands_for_update))
    }

    fn has_subcommand(name: &str) -> bool {
        S::has_subcommand(name)
    }
}

/// Returns `command` with the value name that marks a group not yet
/// expanded, unless the tool already named the value.
fn marked(command: Command) -> Command {
    if command.get_subcommand_value_name().is_some() {
        command
    } else {
        command.subcommand_value_name(MARKER)
    }
}

/// Returns `command` after `augment`, `S`'s function that adds its
/// subcommands, with the texts that `command` already held kept.
///
/// clap's derive applies the field's variant after the group's own
/// `augment_subcommands`, so a variant's doc comment wins over the enum's.
/// A deferred `augment` runs after the variant, so it would replace the
/// variant's texts with the enum's. The function sets each text that the
/// variant gave again, which restores the order.
fn expand(command: Command, augment: fn(Command) -> Command) -> Command {
    let about = command.get_about().cloned();
    let long_about = command.get_long_about().cloned();
    let before_help = command.get_before_help().cloned();
    let before_long_help = command.get_before_long_help().cloned();
    let after_help = command.get_after_help().cloned();
    let after_long_help = command.get_after_long_help().cloned();
    let version = command.get_version().map(str::to_owned);
    let long_version = command.get_long_version().map(str::to_owned);
    let mut command = augment(command);
    if let Some(about) = about {
        command = command.about(about);
    }
    if let Some(long_about) = long_about {
        command = command.long_about(long_about);
    }
    if let Some(text) = before_help {
        command = command.before_help(text);
    }
    if let Some(text) = before_long_help {
        command = command.before_long_help(text);
    }
    if let Some(text) = after_help {
        command = command.after_help(text);
    }
    if let Some(text) = after_long_help {
        command = command.after_long_help(text);
    }
    if let Some(version) = version {
        command = command.version(version);
    }
    if let Some(long_version) = long_version {
        command = command.long_version(long_version);
    }
    command
}
