//! The grammar check, which gives every command the form
//! `<tool> <resource> <verb>`.

use clap::Command;

use crate::tree::{self, DASHED_NAME};

#[cfg(test)]
mod tests;

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
/// hidden ones, and skips each command that `exempt` names and everything
/// below it. An exemption is a command's path below `command`, such as
/// `"schema"`, and an exemption that names no command is a problem. The
/// check finds:
///
/// - a command without subcommands, which is a verb, whose name is not
///   `list`, `show` or one of `verbs`;
/// - a command without subcommands directly under `command`, because every
///   command has the form `<tool> <resource> <verb>`; a tool exempts such a
///   command, such as `schema`;
/// - a command with subcommands, which is a resource, whose name is not
///   lowercase ASCII letters and digits in words joined by single dashes,
///   starting with a letter, such as `reading-lists`. Resources may nest;
/// - an entry of `verbs` that is `list` or `show`, or whose name breaks the
///   same rule as a resource's;
/// - an argument anywhere in the walked tree with the long name `--dry-run`,
///   which crumbkit reserves for its write verbs;
/// - a command below `command` named `help` or with the alias `help`,
///   because crumbkit reserves that word for asking for help. crumbkit's
///   `parse` answers it below every command with subcommands, unless such a
///   command takes it.
///
/// A command that the tool gives to `Command::defer` adds its arguments only
/// when clap builds it, and the check reads the unbuilt tree, so it sees no
/// arguments on such a command and checks none of them.
///
/// ```
/// use clap::Command;
///
/// let shelf = Command::new("shelf")
///     .subcommand(
///         Command::new("books")
///             .subcommand(Command::new("list"))
///             .subcommand(Command::new("export")),
///     )
///     .subcommand(Command::new("schema"));
///
/// crumbkit_test::check_grammar(&shelf, &["export"], &["schema"]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_grammar(command: &Command, verbs: &[&str], exempt: &[&str]) {
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
    for visit in tree::walk(command, exempt, &mut problems) {
        let path = visit.path();
        let command = visit.command();
        let name = command.get_name();
        match (visit.depth(), command.has_subcommands()) {
            // The command passed in is the tool or a subtree's root, which
            // is neither a resource nor a verb.
            (0, _) => {}
            (_, true) => {
                if !tree::is_dashed_name(name) {
                    problems.push(format!(
                        "{path}: the resource name `{name}` is not {DASHED_NAME}"
                    ));
                }
            }
            (1, false) => problems.push(format!(
                "{path}: the command has no subcommands and sits directly under `{tool}`, and a command has the form `{tool} <resource> <verb>`"
            )),
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
        for arg in command.get_arguments() {
            if arg.get_long() == Some("dry-run") {
                problems.push(format!(
                    "{path}: the command has the argument `--dry-run`, and crumbkit reserves that name for its write verbs"
                ));
            }
        }
    }
    tree::report("grammar", &problems);
}
