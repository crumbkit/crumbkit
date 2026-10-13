//! The walk over a command tree, and the helpers that the command tree
//! checks share.

use clap::{Arg, Command};

#[cfg(test)]
mod tests;

/// One command that a walk visits.
pub(crate) struct Visit<'a> {
    command: &'a Command,
    parent: Option<&'a Command>,
    path: String,
    depth: usize,
}

impl<'a> Visit<'a> {
    /// Returns the command.
    pub(crate) fn command(&self) -> &'a Command {
        self.command
    }

    /// Returns the command that holds this command, or `None` for the
    /// command passed to the walk.
    pub(crate) fn parent(&self) -> Option<&'a Command> {
        self.parent
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

    /// Returns the command's names below the command passed to the walk,
    /// separated by single spaces, such as `books list`, as an opt-out or a
    /// node budget names the command. It is empty for the command passed
    /// to the walk.
    pub(crate) fn below(&self) -> &str {
        self.path.split_once(' ').map_or("", |(_, below)| below)
    }
}

/// Returns `root` and every command below it, in depth-first order. The walk
/// includes hidden commands, and `Command::get_arguments` on each visited
/// command includes hidden arguments.
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
pub(crate) fn walk(root: &Command) -> Vec<Visit<'_>> {
    walk_tree(root, true)
}

/// Returns `root` and every visible command below it, as [`walk`] does,
/// leaving out each hidden command and everything below it, as help shows
/// the tree. A command that a tool hides is absent from every help page,
/// and so is every command below it, because no page lists it.
pub(crate) fn walk_visible(root: &Command) -> Vec<Visit<'_>> {
    walk_tree(root, false)
}

/// Returns the commands that [`walk`] returns, or those that
/// [`walk_visible`] returns when `hidden` is unset.
fn walk_tree(root: &Command, hidden: bool) -> Vec<Visit<'_>> {
    let mut visits = Vec::new();
    visit(root, None, root.get_name(), 0, hidden, &mut visits);
    visits
}

/// Adds `command`, which `parent` holds, and every command below it to
/// `visits`. A hidden command and everything below it are left out unless
/// `hidden` is set.
fn visit<'a>(
    command: &'a Command,
    parent: Option<&'a Command>,
    path: &str,
    depth: usize,
    hidden: bool,
    visits: &mut Vec<Visit<'a>>,
) {
    if depth > 0 && !hidden && command.is_hide_set() {
        return;
    }
    visits.push(Visit {
        command,
        parent,
        path: path.to_owned(),
        depth,
    });
    for subcommand in command.get_subcommands() {
        let path = format!("{path} {}", subcommand.get_name());
        visit(subcommand, Some(command), &path, depth + 1, hidden, visits);
    }
}

/// Returns the command that `path`, a sequence of names, leads to from
/// `root`, or `None` when a name matches no subcommand. An empty path leads
/// to `root`. Each name matches a command's name and never its aliases.
pub(crate) fn find<'a, 'n>(
    root: &'a Command,
    path: impl IntoIterator<Item = &'n str>,
) -> Option<&'a Command> {
    let mut command = root;
    for name in path {
        command = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == name)?;
    }
    Some(command)
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
/// problem. The message's first line names the `rules` the tree breaks, and
/// each problem starts on a new line after it. A problem may continue over
/// several lines, such as one that ends with snapbox's report of a golden
/// file that differs.
pub(crate) fn report(rules: &str, problems: &[String]) {
    assert!(
        problems.is_empty(),
        "the command tree breaks crumbkit's {rules}:\n{}",
        problems.join("\n")
    );
}

/// The rule that [`is_dashed_name`] checks, as a problem states it.
pub(crate) const DASHED_NAME: &str =
    "lowercase letters and digits in words joined by single dashes, starting with a letter";

/// Reports whether `name` is lowercase ASCII letters and digits in words
/// joined by single dashes, starting with a letter, such as `reading-lists`.
/// The grammar check applies it to resources and verbs, and the guide check
/// to topic names.
///
/// The check reads only the name's form. It cannot tell a plural noun from a
/// singular one, so it accepts a resource named `book` as readily as `books`,
/// and review checks that each resource is plural. A check for plurals would
/// need an English word list and still miss irregular nouns.
pub(crate) fn is_dashed_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}
