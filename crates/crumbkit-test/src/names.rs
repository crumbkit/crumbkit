//! The names check, which gives every command and argument one name in help,
//! so a caller and an agent learn one spelling.

use clap::Command;

use crate::tree::{self, argument_name};

#[cfg(test)]
mod tests;

/// The rule a command breaks when help shows a second name for it.
const COMMAND_RULE: &str = "and help shows one name for each command";

/// The rule an argument breaks when help shows a second name for it.
const ARGUMENT_RULE: &str = "and help shows one name for each argument";

/// Checks that help shows one name for every command and argument in the
/// tree of `command`, and panics, as a failed test assertion does, naming
/// every problem it finds.
///
/// Help is what agents read and copy. A visible alias costs help tokens on
/// every read and teaches a second spelling. A hidden alias costs neither,
/// and lets a renamed command or flag keep its old name for a while, or
/// accept a common wrong guess. crumbkit's `Next:` lines, recoveries and
/// refusal suggestions print the real name, so a caller who uses a hidden
/// alias still sees the real one.
///
/// The check walks `command` and every subcommand below it, including hidden
/// ones, and skips each command that `exempt` names and everything below it.
/// An exemption is a command's path below `command`, such as `"books list"`,
/// and an exemption that names no command is a problem. The check finds:
///
/// - a command with a visible alias, a visible short-flag alias or a visible
///   long-flag alias;
/// - a command with a short or a long flag name, such as `-B` from
///   `Command::short_flag`, because help lists a flag name beside the
///   command's name;
/// - an argument with a visible long alias or a visible short alias.
///
/// Hidden aliases pass. An argument may have one short form beside its long
/// name, such as `-l` with `--limit`, and clap allows at most one.
///
/// A command that the tool gives to `Command::defer` adds its arguments only
/// when clap builds it, and the check reads the unbuilt tree, so it sees no
/// arguments on such a command and checks none of them.
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
        for alias in command.get_visible_aliases() {
            problems.push(format!(
                "{path}: the command has the visible alias `{alias}`, {COMMAND_RULE}"
            ));
        }
        if let Some(flag) = command.get_short_flag() {
            problems.push(format!(
                "{path}: the command has the short flag name `-{flag}`, {COMMAND_RULE}"
            ));
        }
        if let Some(flag) = command.get_long_flag() {
            problems.push(format!(
                "{path}: the command has the long flag name `--{flag}`, {COMMAND_RULE}"
            ));
        }
        for alias in command.get_visible_short_flag_aliases() {
            problems.push(format!(
                "{path}: the command has the visible short flag alias `-{alias}`, {COMMAND_RULE}"
            ));
        }
        for alias in command.get_visible_long_flag_aliases() {
            problems.push(format!(
                "{path}: the command has the visible long flag alias `--{alias}`, {COMMAND_RULE}"
            ));
        }
        for arg in command.get_arguments() {
            let name = argument_name(arg);
            for alias in arg.get_visible_aliases().unwrap_or_default() {
                problems.push(format!(
                    "{path}: the argument `{name}` has the visible alias `--{alias}`, {ARGUMENT_RULE}"
                ));
            }
            for alias in arg.get_visible_short_aliases().unwrap_or_default() {
                problems.push(format!(
                    "{path}: the argument `{name}` has the visible short alias `-{alias}`, {ARGUMENT_RULE}"
                ));
            }
        }
    }
    tree::report("naming rules", &problems);
}
