//! The names check, which gives every command and argument one name, so a
//! caller and an agent learn one spelling and a retired name fails to parse.

use clap::Command;

use crate::tree::{self, argument_name};

#[cfg(test)]
mod tests;

/// Checks that every command and argument in the tree of `command` has one
/// name, and panics, as a failed test assertion does, naming every problem it
/// finds.
///
/// The check walks `command` and every subcommand below it, including hidden
/// ones, and skips each command that `exempt` names and everything below it.
/// An exemption is a command's path below `command`, such as `"books list"`,
/// and an exemption that names no command is a problem. The check finds:
///
/// - a command with an alias, visible or hidden, or with a short-flag or
///   long-flag alias;
/// - an argument with a long alias or a short alias, visible or hidden.
///
/// An argument may have one short form beside its long name, such as `-l`
/// with `--limit`, and clap allows at most one.
///
/// ```
/// use clap::{Arg, ArgAction, Command};
///
/// let shelf = Command::new("shelf").subcommand(
///     Command::new("books").subcommand(
///         Command::new("list").arg(Arg::new("limit").long("limit").short('l').action(ArgAction::Set)),
///     ),
/// );
///
/// crumbkit_test::check_names(&shelf, &[]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_names(command: &Command, exempt: &[&str]) {
    let mut problems = Vec::new();
    for visit in tree::walk(command, exempt, &mut problems) {
        let path = visit.path();
        let command = visit.command();
        for alias in command.get_all_aliases() {
            problems.push(format!(
                "{path}: the command has the alias `{alias}`, and a command has no aliases"
            ));
        }
        for alias in command.get_all_short_flag_aliases() {
            problems.push(format!(
                "{path}: the command has the short flag alias `-{alias}`, and a command has no aliases"
            ));
        }
        for alias in command.get_all_long_flag_aliases() {
            problems.push(format!(
                "{path}: the command has the long flag alias `--{alias}`, and a command has no aliases"
            ));
        }
        for arg in command.get_arguments() {
            let name = argument_name(arg);
            for alias in arg.get_all_aliases().unwrap_or_default() {
                problems.push(format!(
                    "{path}: the argument `{name}` has the alias `--{alias}`, and an argument has no aliases"
                ));
            }
            for alias in arg.get_all_short_aliases().unwrap_or_default() {
                problems.push(format!(
                    "{path}: the argument `{name}` has the short alias `-{alias}`, and an argument has no aliases"
                ));
            }
        }
    }
    tree::report("naming rules", &problems);
}
