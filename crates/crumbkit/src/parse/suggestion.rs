//! The real names that a refusal suggests in place of the names that clap
//! suggests, which can be hidden aliases, hidden commands or hidden flags.

use clap::error::{ContextKind, ContextValue};
use clap::{Arg, Command};

#[cfg(test)]
mod tests;

/// The start of the tip that clap renders from its suggested flag.
const ARGUMENT: &str = "a similar argument exists: ";
/// The start of the tip that clap renders from one suggested subcommand.
const SUBCOMMAND: &str = "a similar subcommand exists: ";
/// The start of the tip that clap renders from several suggested subcommands.
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
/// walked path, so the last is the command that refused the line. `error`
/// is clap's error, whose context holds the names that clap suggests.
///
/// clap suggests from every name that a flag or a subcommand accepts, hidden
/// or not, so it can suggest a hidden alias, such as `--max` for `--maxx`
/// when `--max` is a hidden alias of `--limit`, or a hidden command. The
/// suggested flag is the `SuggestedArg` context value, and it resolves
/// through [`find_flag`]. The suggested subcommands are the
/// `SuggestedSubcommand` context value, and each resolves to the subcommand
/// of the last command that has that name or alias. clap's tip that names a
/// flag of a subcommand, such as `'books --max' exists`, comes from
/// [`flag_of_subcommand`] and resolves through [`real_flag_of_subcommand`].
///
/// The part that changes is the one that starts with the tip that clap
/// renders from a context value, and only when the error holds that value.
/// Every other part, and a name that resolves to nothing, stay as clap
/// wrote them.
pub(super) fn real_names(part: &str, commands: &[&Command], error: &clap::Error) -> Option<String> {
    let last = commands.last()?;
    if part.starts_with(ARGUMENT)
        && let Some(ContextValue::String(name)) = error.get(ContextKind::SuggestedArg)
    {
        let names = resolve([name.as_str()], |name| match find_flag(name, commands) {
            Some(arg) if arg.is_hide_set() => Resolved::Hidden,
            Some(arg) => arg.get_long().map_or(Resolved::Unknown, |long| {
                Resolved::Visible(format!("--{long}"))
            }),
            None => Resolved::Unknown,
        });
        return (!names.is_empty()).then(|| format!("{ARGUMENT}{}", list(&names)));
    }
    if let Some((name, flag)) = flag_of_subcommand(part, error) {
        return real_flag_of_subcommand(part, name, flag, commands);
    }
    if (part.starts_with(SUBCOMMAND) || part.starts_with(SUBCOMMANDS))
        && let Some(ContextValue::Strings(names)) = error.get(ContextKind::SuggestedSubcommand)
        && !names.is_empty()
    {
        let names = resolve(names.iter().map(String::as_str), |name| {
            match last.find_subcommand(name) {
                Some(subcommand) if subcommand.is_hide_set() => Resolved::Hidden,
                Some(subcommand) => Resolved::Visible(subcommand.get_name().to_owned()),
                None => Resolved::Unknown,
            }
        });
        let start = match names.len() {
            0 => return None,
            1 => SUBCOMMAND,
            _ => SUBCOMMANDS,
        };
        return Some(format!("{start}{}", list(&names)));
    }
    Some(part.to_owned())
}

/// Returns the subcommand and the flag that `part` names when it is clap's
/// tip that a flag belongs to a subcommand, such as `books` and `--max` in
/// `'books --max' exists`, or `None` when `part` is no such tip. `part` is
/// that tip only when it equals the plain text of one of the `Suggested`
/// context values of `error`.
//
// RUST-10: clap 4.6.7 gives this tip only as a finished sentence in
// `ContextKind::Suggested`, so this splits the subcommand and the flag out
// of that one sentence. It misses the tip when clap changes the sentence,
// and the refusal then keeps the tip as clap wrote it. A context value from
// clap that holds the subcommand and the flag as names replaces it.
fn flag_of_subcommand<'a>(part: &'a str, error: &clap::Error) -> Option<(&'a str, &'a str)> {
    let Some(ContextValue::StyledStrs(tips)) = error.get(ContextKind::Suggested) else {
        return None;
    };
    tips.iter().find(|tip| tip.to_string() == part)?;
    part.strip_prefix('\'')?
        .strip_suffix("' exists")?
        .split_once(' ')
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
    let Some(ContextValue::String(suggestion)) = error.get(ContextKind::SuggestedArg) else {
        return None;
    };
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

/// Returns the real name of each of `names` that `real` resolves to a
/// visible flag or subcommand, and each name that it cannot resolve as it
/// is, without repeats and in the order of their first appearance. A name
/// of a hidden flag or subcommand is left out.
fn resolve<'a>(
    names: impl IntoIterator<Item = &'a str>,
    real: impl Fn(&str) -> Resolved,
) -> Vec<String> {
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
