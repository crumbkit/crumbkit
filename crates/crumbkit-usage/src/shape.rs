//! The shape of a command line as clap read it: the command path, the flags
//! typed, the number of ids and the refused flags. Nothing here records a
//! value.

use std::ffi::OsString;

use clap::parser::ValueSource;
use clap::{ArgMatches, Command};

use super::record::{Name, Run};
use crate::events::Refused;

#[cfg(test)]
mod tests;

/// Records in `run` the command path, the flags typed and the number of ids
/// of a command line that clap parsed against `command` into `matches`.
///
/// A flag counts as typed when clap's value source for it is the command
/// line, so a flag left at its default value or read from the environment is
/// not recorded. clap copies a global flag into the matches of every level,
/// and the flag is recorded once.
pub(super) fn parsed(run: &mut Run, command: &Command, matches: &ArgMatches) {
    let mut path: Vec<&Command> = Vec::new();
    let mut flags = Vec::new();
    let mut node = command;
    let mut level = matches;
    loop {
        typed_flags(node, level, &mut flags);
        let Some((name, next)) = level.subcommand() else {
            break;
        };
        let Some(subcommand) = node.find_subcommand(name) else {
            break;
        };
        path.push(subcommand);
        node = subcommand;
        level = next;
    }
    flags.sort();
    flags.dedup();
    run.command = Name::path(&path);
    run.flags = flags;
    run.ids = ids(node, level);
}

/// Records in `run` the command path of the command line that clap
/// `refused`, the number of `words` that look like flags and name no flag of
/// the command, and the visible flag that clap suggested.
///
/// `words` are the words after the tool's name. Unlike clap, which stops at
/// the first unknown flag, the count reads every word before any `--`.
//
// RUST-10: the count reads each word on its own. It counts a value that
// starts with `-`, such as `-5`, as a flag, and it reads only the first
// letter of a group of short flags such as `-ab`. A count that reads how many
// values each option takes, as `parse`'s walk does, replaces it.
pub(super) fn refused(run: &mut Run, refused: &Refused<'_>, words: &[OsString]) {
    let path = subcommands(refused.command(), refused.path());
    let node = path.last().copied().unwrap_or(refused.command());
    let unknown = words
        .iter()
        .take_while(|word| *word != "--")
        .filter(|word| names_no_flag(node, word.to_str()))
        .count();
    run.command = Name::path(&path);
    run.unknown_flags = (unknown > 0).then_some(unknown);
    run.nearest = refused.nearest_flag().and_then(Name::flag);
}

/// Adds to `flags` the name of each flag of `node` that the command line
/// gave a value in `matches`.
fn typed_flags(node: &Command, matches: &ArgMatches, flags: &mut Vec<Name>) {
    for id in matches.ids() {
        if matches.value_source(id.as_str()) != Some(ValueSource::CommandLine) {
            continue;
        }
        // The id of a group names no argument, and a positional argument
        // has no flag name.
        let flag = node
            .get_arguments()
            .find(|arg| arg.get_id() == id)
            .and_then(Name::flag);
        if let Some(flag) = flag {
            flags.push(flag);
        }
    }
}

/// Returns the subcommands of `command` that `path` names, one level below
/// the other, and leaves out a name that names no subcommand at its level.
pub(super) fn subcommands<'a>(command: &'a Command, path: &[String]) -> Vec<&'a Command> {
    let mut found = Vec::new();
    let mut node = command;
    for name in path {
        if let Some(subcommand) = node.find_subcommand(name) {
            found.push(subcommand);
            node = subcommand;
        }
    }
    found
}

/// Returns how many values the positional arguments of `node` whose value
/// name is `ID` received in `matches`, or `None` when `node` has no such
/// argument.
fn ids(node: &Command, matches: &ArgMatches) -> Option<usize> {
    let mut count = None;
    for arg in node.get_positionals() {
        let named_id = arg
            .get_value_names()
            .is_some_and(|names| names.iter().any(|name| name == "ID"));
        if !named_id {
            continue;
        }
        let values = matches
            .try_get_raw(arg.get_id().as_str())
            .ok()
            .flatten()
            .map_or(0, |values| values.len());
        count = Some(count.unwrap_or(0) + values);
    }
    count
}

/// Returns whether `word` looks like a flag and names no flag of `node`,
/// by its long name, its short name or an alias. A word that is not UTF-8
/// is not counted.
fn names_no_flag(node: &Command, word: Option<&str>) -> bool {
    let Some(word) = word else {
        return false;
    };
    if let Some(long) = word.strip_prefix("--") {
        let name = long.split_once('=').map_or(long, |(name, _)| name);
        return !node.get_arguments().any(|arg| {
            arg.get_long() == Some(name)
                || arg
                    .get_all_aliases()
                    .is_some_and(|aliases| aliases.contains(&name))
        });
    }
    let Some(first) = word
        .strip_prefix('-')
        .and_then(|short| short.chars().next())
    else {
        return false;
    };
    !node.get_arguments().any(|arg| {
        arg.get_short() == Some(first)
            || arg
                .get_all_short_aliases()
                .is_some_and(|aliases| aliases.contains(&first))
    })
}
