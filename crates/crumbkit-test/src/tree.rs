//! The walk over a command tree that the command tree checks share.

use clap::{Arg, Command};

#[cfg(test)]
mod tests;

/// One command that a walk visits.
pub(crate) struct Visit<'a> {
    command: &'a Command,
    path: String,
    depth: usize,
}

impl<'a> Visit<'a> {
    /// Returns the command.
    pub(crate) fn command(&self) -> &'a Command {
        self.command
    }

    /// Returns the command's names from the command passed to the walk down
    /// to this command, separated by single spaces, such as
    /// `shelf books list`.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// Returns how many levels the command sits below the command passed to
    /// the walk, which is at depth 0.
    pub(crate) fn depth(&self) -> usize {
        self.depth
    }
}

/// Returns `root` and every command below it, in depth-first order, leaving
/// out each command that `exempt` names and everything below it. It adds a
/// problem to `problems` for each exemption that names no command, so the
/// exemption of a removed command cannot stay in a tool's list.
///
/// An exemption is a command's path below `root`, with its names separated
/// by single spaces, such as `books list`. The walk includes hidden commands,
/// and `Command::get_arguments` on each visited command includes hidden
/// arguments.
///
/// The walk reads the tree as the tool built it and never calls
/// `Command::build`. clap's build adds a `help` subcommand to every command
/// that has subcommands, adds `--help` and `--version` arguments, and copies
/// each `global = true` argument onto every command below the one that
/// defines it. A check of the built tree would report clap's own additions as
/// the tool's commands and report each global argument once for every
/// command, so the walk leaves the tree unbuilt. A command that the tool
/// gives to `Command::defer` adds its arguments only when it is built, so the
/// checks see no arguments on it.
pub(crate) fn walk<'a>(
    root: &'a Command,
    exempt: &[&str],
    problems: &mut Vec<String>,
) -> Vec<Visit<'a>> {
    for &exemption in exempt {
        if !names_command(root, exemption) {
            problems.push(format!("the exemption `{exemption}` names no command"));
        }
    }
    let mut visits = Vec::new();
    visit(root, "", root.get_name(), 0, exempt, &mut visits);
    visits
}

/// Adds `command` and every command below it to `visits`, unless `exempt`
/// names `below`, the command's path below the root of the walk, which is
/// empty for the root itself.
fn visit<'a>(
    command: &'a Command,
    below: &str,
    path: &str,
    depth: usize,
    exempt: &[&str],
    visits: &mut Vec<Visit<'a>>,
) {
    if depth > 0 && exempt.contains(&below) {
        return;
    }
    visits.push(Visit {
        command,
        path: path.to_owned(),
        depth,
    });
    for subcommand in command.get_subcommands() {
        let name = subcommand.get_name();
        let below = if depth == 0 {
            name.to_owned()
        } else {
            format!("{below} {name}")
        };
        visit(
            subcommand,
            &below,
            &format!("{path} {name}"),
            depth + 1,
            exempt,
            visits,
        );
    }
}

/// Reports whether `exemption`, a path of names separated by single spaces,
/// leads from `root` to a command. Each name matches a command's name and
/// never its aliases.
fn names_command(root: &Command, exemption: &str) -> bool {
    let mut command = root;
    for name in exemption.split(' ') {
        match command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == name)
        {
            Some(subcommand) => command = subcommand,
            None => return false,
        }
    }
    true
}

/// Returns the name a caller types for `arg`: `--limit` for an argument with
/// a long name, `-l` for one with only a short name, and the argument's id,
/// such as `ids`, for a positional argument.
pub(crate) fn argument_name(arg: &Arg) -> String {
    if let Some(long) = arg.get_long() {
        format!("--{long}")
    } else if let Some(short) = arg.get_short() {
        format!("-{short}")
    } else {
        arg.get_id().to_string()
    }
}

/// Reports whether `arg` takes values on the command line: its action takes
/// values and its `num_args`, when the tool set it, accepts at least one.
///
/// Before `Command::build`, an argument has no action and no `num_args`
/// unless the tool set them, and `Arg::get_action` then returns
/// `ArgAction::Set`. The build gives an argument with `num_args(0)` and no
/// action `ArgAction::SetTrue`, so this reads `num_args` too.
pub(crate) fn takes_values(arg: &Arg) -> bool {
    arg.get_action().takes_values() && arg.get_num_args().is_none_or(|range| range.takes_values())
}

/// Panics, as a failed test assertion does, when `problems` holds any
/// problem, with one problem on each line after a first line that names the
/// `rules` the tree breaks.
pub(crate) fn report(rules: &str, problems: &[String]) {
    assert!(
        problems.is_empty(),
        "the command tree breaks crumbkit's {rules}:\n{}",
        problems.join("\n")
    );
}
