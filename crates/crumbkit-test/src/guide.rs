//! The guide check, which keeps a tool's goal headings in step with its
//! command tree.

use clap::Command;
use crumbkit::help::Guide;

use crate::tree;

#[cfg(test)]
mod tests;

/// Checks that `guide` names the commands of the tree of `command` as
/// `crumbkit::help::extend` lists them, and panics, as a failed test
/// assertion does, naming every problem it finds.
///
/// `extend` never fails at runtime: it skips a group or a name that matches
/// no command, and lists a visible subcommand that no goal names under a
/// final `Commands` heading. This check catches each of those cases in a
/// test instead. A group's path runs from `command`, and the check reads
/// the tree as the tool built it, so clap's generated `help` subcommand is
/// never a child. It finds:
///
/// - a group whose path names no command, matching real names only;
/// - a group whose path names a command without subcommands;
/// - a goal that names a command that is not a subcommand of its group,
///   such as an alias or a misspelt name;
/// - a visible subcommand of a group that no goal names;
/// - a subcommand that two goals of its group name, or that one goal names
///   twice.
///
/// A hidden subcommand passes whether or not a goal names it, and a command
/// that no group names keeps clap's list and is not checked. The check
/// takes no exemptions, because a tool that wants clap's list for a command
/// leaves that command out of the guide.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::{Goal, Group, Guide};
///
/// const GUIDE: Guide = Guide::new().groups(&[Group::new(
///     &["books"],
///     &[Goal::new("Read", &["list", "show"])],
/// )]);
///
/// let shelf = Command::new("shelf").subcommand(
///     Command::new("books")
///         .subcommand(Command::new("list"))
///         .subcommand(Command::new("show")),
/// );
///
/// crumbkit_test::check_guide(&shelf, &GUIDE);
/// ```
///
/// # Panics
///
/// Panics when the guide breaks a rule above. The message lists each
/// problem on its own line and names each command by its path from
/// `command`, such as `shelf books`.
pub fn check_guide(command: &Command, guide: &Guide) {
    let mut problems = Vec::new();
    for group in guide.get_groups() {
        let path = group.path();
        let Some(node) = tree::find(command, path.iter().copied()) else {
            problems.push(format!("the group `{}` names no command", path.join(" ")));
            continue;
        };
        let text = std::iter::once(command.get_name())
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if !node.has_subcommands() {
            problems.push(format!(
                "{text}: the guide gives the command goals, and it has no subcommands to list under them"
            ));
            continue;
        }
        for goal in group.goals() {
            for &name in goal.commands() {
                if tree::find(node, [name]).is_none() {
                    problems.push(format!(
                        "{text}: the goal `{}` names `{name}`, which is not a subcommand of the command",
                        goal.heading()
                    ));
                }
            }
        }
        for subcommand in node.get_subcommands() {
            let name = subcommand.get_name();
            let headings: Vec<String> = group
                .goals()
                .iter()
                .flat_map(|goal| {
                    goal.commands()
                        .iter()
                        .filter(move |&&named| named == name)
                        .map(|_| format!("`{}`", goal.heading()))
                })
                .collect();
            match headings.as_slice() {
                [] if !subcommand.is_hide_set() => problems.push(format!(
                    "{text}: no goal names the visible subcommand `{name}`, so help lists it under a final `Commands` heading"
                )),
                [] | [_] => {}
                [first @ .., last] => problems.push(format!(
                    "{text}: the goals {} and {last} name `{name}`, so help lists it under each of them",
                    first.join(", ")
                )),
            }
        }
    }
    tree::report("goal heading rules", &problems);
}
