//! `reach`, which expands the lazy groups that the typed words pass through
//! before anything reads the tree (ADR-0246).

use std::ffi::OsStr;

use clap::Command;

use super::lines::hide_global_flags;
use super::walk::given;
use crate::lazy::MARKER;

#[cfg(test)]
mod tests;

/// Expands each [`Lazy`](crate::Lazy) group that `words`, the words after
/// the program name, pass through in `command`, so a reader of the tree finds
/// the group's subcommands and their arguments, and returns.
///
/// It sets `disable_help_subcommand(true)` on the root first, because clap
/// adds its `help` subcommand to every command it builds otherwise, and
/// crumbkit answers `help` itself. It then builds the commands from the root
/// down to each group and the group last, one command at a time. Building the
/// root before the group matters: clap copies a command's global arguments
/// only into its direct subcommands when it builds that command, and builds
/// no command twice, so a group built before its parent misses the root's
/// global flags, such as `--json`.
///
/// Each command it builds first hides its visible global flags from `-h`,
/// as the page's `Global flags:` line expects, so the copies that clap gives
/// the subcommands are hidden too.
///
/// A line that types no lazy group's word builds nothing. A word that names
/// no command, and each word after `--`, ends the search as
/// [`command_path`](crate::command_path) reads it.
pub fn reach<T: AsRef<OsStr>>(command: &mut Command, words: &[T]) {
    loop {
        let path = given(command, words).path;
        if !unexpanded(command, &path) {
            return;
        }
        *command = std::mem::take(command).disable_help_subcommand(true);
        build_down(command, &path);
        // A marked command that gained no subcommands is no group, and
        // the next pass would read the same path.
        if unexpanded(command, &path) {
            return;
        }
    }
}

/// Expands every lazy group below `command`, as [`reach`] expands the groups
/// that typed words pass through. A reader of the whole tree that does not
/// call `Command::build`, such as `help::extend`, calls it first.
pub(crate) fn reach_all(command: &mut Command) {
    // A marked command that gains no subcommands is no group, so it is
    // tried once.
    let mut tried: Vec<Vec<String>> = Vec::new();
    if first_unexpanded(command, &mut Vec::new(), &tried).is_none() {
        return;
    }
    *command = std::mem::take(command).disable_help_subcommand(true);
    while let Some(path) = first_unexpanded(command, &mut Vec::new(), &tried) {
        build_down(command, &path);
        tried.push(path);
    }
}

/// Returns a clone of `command` with every lazy group expanded, or `None`
/// when `command` holds no lazy group to expand. crumbkit-test's checks walk
/// the clone, so they find the leaves of a lazy group.
#[doc(hidden)]
#[must_use]
pub fn expanded_clone(command: &Command) -> Option<Command> {
    first_unexpanded(command, &mut Vec::new(), &[])?;
    let mut clone = command.clone();
    reach_all(&mut clone);
    Some(clone)
}

/// Returns the path of the first lazy group below `node` with no
/// subcommands that `tried` does not hold, depth first, where `path` is the
/// path of `node`.
fn first_unexpanded(
    node: &Command,
    path: &mut Vec<String>,
    tried: &[Vec<String>],
) -> Option<Vec<String>> {
    if node.get_subcommand_value_name() == Some(MARKER)
        && !node.has_subcommands()
        && !tried.contains(path)
    {
        return Some(path.clone());
    }
    for child in node.get_subcommands() {
        path.push(child.get_name().to_owned());
        let found = first_unexpanded(child, path, tried);
        path.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Reports whether the command at `path` below `command` is a lazy group
/// that has no subcommands yet.
fn unexpanded(command: &Command, path: &[String]) -> bool {
    let mut node = command;
    for name in path {
        let Some(next) = node.find_subcommand(name) else {
            return false;
        };
        node = next;
    }
    node.get_subcommand_value_name() == Some(MARKER) && !node.has_subcommands()
}

/// Builds the root `command` and each command below it down to the one at
/// `path`, the root first.
fn build_down(command: &mut Command, path: &[String]) {
    let mut node = command;
    build(node);
    for name in path {
        let Some(next) = node.find_subcommand_mut(name) else {
            return;
        };
        node = next;
        build(node);
    }
}

/// Hides the visible global flags of `node` from `-h`, as the help lines do
/// before they build a command, and builds `node` alone with
/// `Command::render_usage`.
fn build(node: &mut Command) {
    hide_global_flags(node);
    let _ = node.render_usage();
}
