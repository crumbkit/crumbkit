//! The grammar check, which gives every command the form
//! `<tool> <resource> <verb>`.

use clap::Command;

use crate::allow::{Allow, Allowances, Rule};
use crate::tree::{self, DASHED_NAME};

#[cfg(test)]
mod tests;

/// The rules of the grammar check that a tool can opt out of.
const RULES: [Rule; 2] = [Rule::RootLeaf, Rule::ResourceName];

/// The verbs that every tool has, which a tool does not pass to
/// [`check_grammar`].
const BUILT_IN_VERBS: [&str; 2] = ["list", "show"];

/// Checks that every command in the tree of `command` is a resource or a
/// verb, and panics, as a failed test assertion does, naming every problem it
/// finds.
///
/// `command` is the tool, or the root of a subtree, and is checked as neither
/// a resource nor a verb. `verbs` holds the tool's verbs beside `list` and
/// `show`. The check walks `command` and every subcommand below it, including
/// hidden ones. It finds:
///
/// - a command without subcommands, which is a verb, whose name is not
///   `list`, `show` or one of `verbs`;
/// - a command without subcommands directly under `command`, because every
///   command has the form `<tool> <resource> <verb>`, unless an opt-out from
///   [`Rule::RootLeaf`] in `allow` names it, such as `version`. The verb
///   rule above skips such a leaf;
/// - a command with subcommands, which is a resource, whose name is not
///   lowercase ASCII letters and digits in words joined by single dashes,
///   starting with a letter, such as `reading-lists`, unless an opt-out
///   from [`Rule::ResourceName`] covers it. Resources may nest;
/// - an entry of `verbs` that is `list` or `show`, or whose name breaks the
///   same rule as a resource's;
/// - a command below `command` named `help` or with the alias `help`,
///   because crumbkit reserves that word for asking for help. crumbkit's
///   `parse` answers it below every command with subcommands, unless such a
///   command takes it, so this rule has no opt-out;
/// - an opt-out from [`Rule::RootLeaf`] or [`Rule::ResourceName`] that
///   names no command or excuses no problem.
///
/// An opt-out covers the command it names and every command below it, and
/// the check reads only the opt-outs of its own rules from `allow`.
///
/// [`check_effects`](crate::check_effects) checks `--dry-run`.
///
/// ```
/// use clap::Command;
/// use crumbkit_test::{Allow, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(
///     Rule::RootLeaf,
///     "version",
///     "People type `shelf version` from habit.",
/// )];
///
/// let shelf = Command::new("shelf")
///     .subcommand(
///         Command::new("books")
///             .subcommand(Command::new("list"))
///             .subcommand(Command::new("export")),
///     )
///     .subcommand(Command::new("version"));
///
/// crumbkit_test::check_grammar(&shelf, &["export"], ALLOW);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_grammar(command: &Command, verbs: &[&str], allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &RULES);
    let mut problems = Vec::new();
    for &verb in verbs {
        if BUILT_IN_VERBS.contains(&verb) {
            problems.push(format!(
                "the tool passes the verb `{verb}`, which is built in, and a tool passes only its other verbs"
            ));
        } else if !tree::is_dashed_name(verb) {
            problems.push(format!("the verb `{verb}` is not {DASHED_NAME}"));
        }
    }
    let tool = command.get_name();
    for visit in tree::walk(command) {
        let path = visit.path();
        let below = visit.below();
        let command = visit.command();
        let name = command.get_name();
        match (visit.depth(), command.has_subcommands()) {
            // The command passed in is the tool or a subtree's root, which
            // is neither a resource nor a verb.
            (0, _) => {}
            (_, true) => {
                if !tree::is_dashed_name(name) && !allowances.excuses(Rule::ResourceName, below) {
                    problems.push(format!(
                        "{path}: the resource name `{name}` is not {DASHED_NAME}"
                    ));
                }
            }
            (1, false) => {
                if !allowances.excuses(Rule::RootLeaf, below) {
                    problems.push(format!(
                        "{path}: the command has no subcommands and sits directly under `{tool}`, and a command has the form `{tool} <resource> <verb>`"
                    ));
                }
            }
            (_, false) => {
                if !BUILT_IN_VERBS.contains(&name) && !verbs.contains(&name) {
                    problems.push(format!(
                        "{path}: the command has no subcommands, so it is a verb, but `{name}` is not `list`, `show` or a verb the tool passes"
                    ));
                }
            }
        }
        if visit.depth() > 0 && (name == "help" || command.get_all_aliases().any(|a| a == "help")) {
            problems.push(format!(
                "{path}: the command answers to `help`, and crumbkit reserves that word for asking for help"
            ));
        }
    }
    allowances.report(command, "grammar", problems);
}
