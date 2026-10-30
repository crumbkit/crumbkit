//! The real names that a refusal suggests in place of the names that clap
//! suggests, which can be hidden aliases, hidden commands or hidden flags.

use clap::{Arg, Command};

#[cfg(test)]
mod tests;

/// The start of clap's tip that suggests one flag.
const ARGUMENT: &str = "a similar argument exists: ";
/// The start of clap's tip that suggests one subcommand.
const SUBCOMMAND: &str = "a similar subcommand exists: ";
/// The start of clap's tip that suggests several subcommands.
const SUBCOMMANDS: &str = "some similar subcommands exist: ";

/// What a name that clap suggested resolves to.
enum Resolved {
    /// The real name of a visible flag or subcommand.
    Visible(String),
    /// A hidden flag or subcommand, which the refusal leaves out.
    Hidden,
    /// Nothing that crumbkit can find, so the name stays as clap wrote it.
    Unknown,
}

/// Returns `part`, one part of a refusal's message, with each name that
/// clap's tip suggests replaced by the real name that it belongs to, each
/// repeated name left out, and each name of a hidden flag or subcommand left
/// out. It returns `None` when the tip suggests nothing else, so the
/// refusal drops the tip. `commands` holds the root and each command on the
/// walked path, so the last is the command that refused the line.
///
/// clap suggests from every name that a flag or a subcommand accepts, hidden
/// or not, so it can suggest a hidden alias, such as `--max` for `--maxx`
/// when `--max` is a hidden alias of `--limit`, or a hidden command. A
/// suggested flag resolves through [`find_flag`]. A suggested subcommand
/// resolves to the subcommand of the last command that has that name or
/// alias. clap's tip that names a flag of a subcommand, such as `'books
/// --max' exists`, resolves through [`real_flag_of_subcommand`]. A part that
/// is no such tip, and a name that resolves to nothing, stay as clap wrote
/// them.
//
// RUST-10: this reads clap's wording of the tips, as `message` does. It
// misses a suggestion when clap changes the wording. crumbkit enables clap's
// `error-context` feature (ADR-0003), so reading `ContextKind::SuggestedArg`,
// `ContextKind::SuggestedSubcommand` and `ContextKind::Suggested` through
// `Error::get` can replace it.
pub(super) fn real_names(part: &str, commands: &[&Command]) -> Option<String> {
    let last = commands.last()?;
    if let Some(names) = part.strip_prefix(ARGUMENT).and_then(quoted) {
        let names = resolve(names, |name| match find_flag(name, commands) {
            Some(arg) if arg.is_hide_set() => Resolved::Hidden,
            Some(arg) => arg.get_long().map_or(Resolved::Unknown, |long| {
                Resolved::Visible(format!("--{long}"))
            }),
            None => Resolved::Unknown,
        });
        return (!names.is_empty()).then(|| format!("{ARGUMENT}{}", list(&names)));
    }
    let flag_of_subcommand = part
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix("' exists"))
        .and_then(|words| words.split_once(' '));
    if let Some((name, flag)) = flag_of_subcommand {
        return real_flag_of_subcommand(part, name, flag, commands);
    }
    let names = part
        .strip_prefix(SUBCOMMAND)
        .or_else(|| part.strip_prefix(SUBCOMMANDS))
        .and_then(quoted);
    let Some(names) = names else {
        return Some(part.to_owned());
    };
    let names = resolve(names, |name| match last.find_subcommand(name) {
        Some(subcommand) if subcommand.is_hide_set() => Resolved::Hidden,
        Some(subcommand) => Resolved::Visible(subcommand.get_name().to_owned()),
        None => Resolved::Unknown,
    });
    let start = match names.len() {
        0 => return None,
        1 => SUBCOMMAND,
        _ => SUBCOMMANDS,
    };
    Some(format!("{start}{}", list(&names)))
}

/// Returns `part`, clap's tip that the flag `flag` of the subcommand `name`
/// exists, such as `'books --max' exists`, with the real long name of the
/// flag, or `None` when the subcommand or the flag is hidden. The
/// subcommand is the first that a command in `commands` has, the last
/// first. A tip whose subcommand or flag crumbkit cannot find stays as clap
/// wrote it.
fn real_flag_of_subcommand(
    part: &str,
    name: &str,
    flag: &str,
    commands: &[&Command],
) -> Option<String> {
    let Some(subcommand) = commands
        .iter()
        .rev()
        .find_map(|command| command.find_subcommand(name))
    else {
        return Some(part.to_owned());
    };
    if subcommand.is_hide_set() {
        return None;
    }
    match find_flag(flag, &[subcommand]) {
        Some(arg) if arg.is_hide_set() => None,
        Some(arg) => match arg.get_long() {
            Some(long) => Some(format!("'{} --{long}' exists", subcommand.get_name())),
            None => Some(part.to_owned()),
        },
        None => Some(part.to_owned()),
    }
}

/// Returns the visible argument that clap suggested in `error` for a flag
/// it refused, such as the argument of `--all` for `--al`, or `None` when
/// clap suggested none, as [`RefusalEvent::nearest_flag`] describes. `command`
/// is the tool's root, and `path` the command path that `parse` walked. A
/// word of `path` that names no subcommand is left out.
///
/// [`RefusalEvent::nearest_flag`]: crate::events::RefusalEvent::nearest_flag
//
// RUST-10: this reads the suggestion from clap's rendered text, as
// `real_names` does. It misses a suggestion when clap changes the wording,
// and then returns `None`. Reading `ContextKind::SuggestedArg` through
// `Error::get` can replace it.
pub(crate) fn nearest_flag<'a>(
    command: &'a Command,
    path: &[String],
    error: &clap::Error,
) -> Option<&'a Arg> {
    let mut commands = vec![command];
    for name in path {
        if let Some(subcommand) = commands.last().and_then(|node| node.find_subcommand(name)) {
            commands.push(subcommand);
        }
    }
    let text = error.render().to_string();
    let (_, rest) = text.split_once("a similar argument exists: '")?;
    let (suggestion, _) = rest.split_once('\'')?;
    find_flag(suggestion, &commands).filter(|arg| !arg.is_hide_set())
}

/// Returns the argument that the flag `name`, such as `--max`, names by its
/// long name or an alias: an argument of the last of `commands`, or else a
/// global argument of a command above it, nearest first, as clap copies
/// global arguments down. `commands` holds the root and each command on the
/// path below it. The argument may be hidden.
fn find_flag<'a>(name: &str, commands: &[&'a Command]) -> Option<&'a Arg> {
    let long = name.strip_prefix("--")?;
    let (last, above) = commands.split_last()?;
    let names_flag = |arg: &&Arg| {
        arg.get_long() == Some(long)
            || arg
                .get_all_aliases()
                .is_some_and(|aliases| aliases.contains(&long))
    };
    let own = last.get_arguments().find(names_flag);
    own.or_else(|| {
        above
            .iter()
            .rev()
            .flat_map(|command| command.get_arguments())
            .filter(|arg| arg.is_global_set())
            .find(names_flag)
    })
}

/// Returns the names in `text`, a list such as `'books', 'book'`, or `None`
/// when a name in it is not in single quotes.
fn quoted(text: &str) -> Option<Vec<&str>> {
    text.split(", ")
        .map(|name| name.strip_prefix('\'')?.strip_suffix('\''))
        .collect()
}

/// Returns the real name of each of `names` that `real` resolves to a
/// visible flag or subcommand, and each name that it cannot resolve as it
/// is, without repeats and in the order of their first appearance. A name
/// of a hidden flag or subcommand is left out.
fn resolve(names: Vec<&str>, real: impl Fn(&str) -> Resolved) -> Vec<String> {
    let mut resolved: Vec<String> = Vec::new();
    for name in names {
        let name = match real(name) {
            Resolved::Visible(real) => real,
            Resolved::Hidden => continue,
            Resolved::Unknown => name.to_owned(),
        };
        if !resolved.contains(&name) {
            resolved.push(name);
        }
    }
    resolved
}

/// Returns `names` in single quotes, separated by commas, as clap lists
/// them.
fn list(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ")
}
