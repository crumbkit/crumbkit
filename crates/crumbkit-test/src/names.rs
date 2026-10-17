//! The names check, which gives every command and argument one name in help,
//! so a caller and an agent learn one spelling.

use crate::Pages;
use crate::allow::{Allow, Allowances, Rule};
use crate::tree::{self, argument_name};
use clap::Command;

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
/// The check covers what help shows: `command`, every visible subcommand
/// below it and every visible argument of each. It skips each hidden command
/// and everything below it and each hidden argument, because no help page
/// shows them. It finds the problems below, except on a command that an
/// opt-out from [`Rule::OneName`] in `allow` covers: the command it names
/// and every command below it. It finds:
///
/// - a command with a visible alias, a visible short-flag alias or a visible
///   long-flag alias;
/// - a command with a short or a long flag name, such as `-B` from
///   `Command::short_flag`, because help lists a flag name beside the
///   command's name;
/// - an argument with a visible long alias or a visible short alias;
/// - a guide in `pages` that names a person's aliases on the root's help
///   with `Guide::personal_aliases`, unless an opt-out from
///   [`Rule::PersonalAliases`] names the root, because those names differ
///   between machines (ADR-0027);
/// - an opt-out from [`Rule::OneName`] or [`Rule::PersonalAliases`] that
///   names no command or excuses no problem.
///
/// Hidden aliases pass, and so does a visible alias of a hidden command or
/// argument. An argument may have one short form beside its long name, such
/// as `-l` with `--limit`, and clap allows at most one.
///
/// A command that the tool gives to `Command::defer` adds its arguments only
/// when clap builds it, and the check reads the unbuilt tree, so it sees no
/// arguments on such a command and checks none of them.
///
/// `pages` gives the check the tool's guide: [`Pages::rendered`] and
/// [`Pages::binary`] carry it, and [`Pages::clap`], for a tool that parses
/// with clap alone, carries none. The check reads no help page.
///
/// ```
/// use clap::{Arg, ArgAction, Command};
/// use crumbkit_test::Pages;
///
/// let shelf = Command::new("shelf").subcommand(
///     Command::new("books").subcommand(
///         Command::new("list").arg(Arg::new("limit").long("limit").short('l').action(ArgAction::Set)),
///     ),
/// );
///
/// crumbkit_test::check_names(&shelf, Pages::clap(), &[]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_names(command: &Command, pages: Pages<'_>, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::OneName, Rule::PersonalAliases]);
    let mut problems = Vec::new();
    let personal_aliases = pages
        .guide()
        .is_some_and(|guide| guide.get_personal_aliases().is_some());
    if personal_aliases && !allowances.excuses(Rule::PersonalAliases, "") {
        problems.push(format!(
            "{}: the guide names a person's aliases on the root's help, {COMMAND_RULE}",
            command.get_name()
        ));
    }
    for visit in tree::walk_visible(command) {
        let path = visit.path();
        let command = visit.command();
        // The problems of this command, which an opt-out may excuse.
        let mut found = Vec::new();
        for alias in command.get_visible_aliases() {
            found.push(format!(
                "{path}: the command has the visible alias `{alias}`, {COMMAND_RULE}"
            ));
        }
        if let Some(flag) = command.get_short_flag() {
            found.push(format!(
                "{path}: the command has the short flag name `-{flag}`, {COMMAND_RULE}"
            ));
        }
        if let Some(flag) = command.get_long_flag() {
            found.push(format!(
                "{path}: the command has the long flag name `--{flag}`, {COMMAND_RULE}"
            ));
        }
        for alias in command.get_visible_short_flag_aliases() {
            found.push(format!(
                "{path}: the command has the visible short flag alias `-{alias}`, {COMMAND_RULE}"
            ));
        }
        for alias in command.get_visible_long_flag_aliases() {
            found.push(format!(
                "{path}: the command has the visible long flag alias `--{alias}`, {COMMAND_RULE}"
            ));
        }
        for arg in command.get_arguments().filter(|arg| !arg.is_hide_set()) {
            let name = argument_name(arg);
            for alias in arg.get_visible_aliases().unwrap_or_default() {
                found.push(format!(
                    "{path}: the argument `{name}` has the visible alias `--{alias}`, {ARGUMENT_RULE}"
                ));
            }
            for alias in arg.get_visible_short_aliases().unwrap_or_default() {
                found.push(format!(
                    "{path}: the argument `{name}` has the visible short alias `-{alias}`, {ARGUMENT_RULE}"
                ));
            }
        }
        if !found.is_empty() && !allowances.excuses(Rule::OneName, visit.below()) {
            problems.append(&mut found);
        }
    }
    allowances.report(command, "naming rules", problems);
}
