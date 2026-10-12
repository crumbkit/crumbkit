//! The effects check, which requires every leaf to declare its effects and
//! gives `--dry-run`, `--yes` and `--no-input` one meaning in every tool.

use clap::Command;
use crumbkit::help::{Effects, Guide, Leaf};

use crate::allow::{Allow, Allowances, Rule, words};
use crate::tree;

#[cfg(test)]
mod tests;

/// The rules of the effects check that a tool can opt out of.
const RULES: [Rule; 4] = [Rule::Effects, Rule::DryRun, Rule::Yes, Rule::NoInput];

/// The id that `crumbkit::DryRun` gives its `--dry-run`, which tells it
/// apart from a tool's own flag with the same name.
const CRUMBKIT_DRY_RUN: &str = "crumbkit_dry_run";

/// The id that `crumbkit::NoInput` gives its `--no-input`.
const CRUMBKIT_NO_INPUT: &str = "crumbkit_no_input";

/// Checks that every leaf in the tree of `command` declares its effects in
/// `guide`, and that `--dry-run`, `--yes` and `--no-input` sit where crumbkit's
/// conventions put them, and panics, as a failed test assertion does,
/// naming every problem it finds.
///
/// A leaf is a command without subcommands, the root included when it has
/// none, and its effects are those of the first [`Leaf`] in `guide` with
/// its path, which [`Leaf::effects`] sets. The check finds:
///
/// - a visible leaf without effects, unless an opt-out from
///   [`Rule::Effects`] covers it. An agent reads such a leaf's effects as
///   unknown and must assume that it may destroy data;
/// - crumbkit's `--dry-run`, which `crumbkit::DryRun` adds, on a read-only
///   leaf, because such a leaf has nothing to skip. This rule has no
///   opt-out;
/// - an argument anywhere in the tree with the long name `--dry-run` that
///   `crumbkit::DryRun` did not add, unless an opt-out from
///   [`Rule::DryRun`] covers its command, because crumbkit's flag marks a
///   dry run's document with `dry_run` and a tool's own flag does not;
/// - a destructive leaf without an argument with the long name `--yes`,
///   unless an opt-out from [`Rule::Yes`] covers it, because
///   `Output::confirm` fails without a terminal unless the caller passes
///   `--yes`;
/// - a tool that can ask a question, because `guide` has a picker or a
///   command has an argument with the long name `--yes`, without the
///   `--no-input` of `crumbkit::NoInput` at the root, unless an opt-out
///   from [`Rule::NoInput`] names the root, because a script or an agent
///   then has no way to say that nobody can answer;
/// - an opt-out from one of the four rules above that names no command or
///   excuses no problem.
///
/// An opt-out covers the command it names and every command below it, and
/// the check reads only the opt-outs of its own rules from `allow`. The
/// check reads the unbuilt tree, as the grammar check does, so it sees no
/// arguments on a command that the tool gives to `Command::defer`.
///
/// ```
/// use clap::{Arg, ArgAction, Command};
/// use crumbkit::help::{Effects, Guide, Leaf};
///
/// const GUIDE: Guide = Guide::new().leaves(&[
///     Leaf::new(&["books", "list"]).effects(Effects::read_only()),
///     Leaf::new(&["books", "remove"]).effects(Effects::destructive().idempotent()),
/// ]);
///
/// let no_input = Arg::new("crumbkit_no_input").long("no-input").global(true).action(ArgAction::SetTrue);
/// let shelf = Command::new("shelf").arg(no_input).subcommand(
///     Command::new("books")
///         .subcommand(Command::new("list"))
///         .subcommand(
///             Command::new("remove").arg(Arg::new("yes").long("yes").action(ArgAction::SetTrue)),
///         ),
/// );
///
/// crumbkit_test::check_effects(&shelf, &GUIDE, &[]);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books remove`.
///
/// [`Leaf`]: crumbkit::help::Leaf
/// [`Leaf::effects`]: crumbkit::help::Leaf::effects
pub fn check_effects(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &RULES);
    let mut problems = Vec::new();
    let mut asks = guide.get_picker().is_some();
    for visit in tree::walk(command) {
        let path = visit.path();
        let below = visit.below();
        let node = visit.command();
        let effects = if node.has_subcommands() {
            None
        } else {
            effects_of(guide, below)
        };
        if !node.has_subcommands()
            && effects.is_none()
            && is_visible(command, below)
            && !allowances.excuses(Rule::Effects, below)
        {
            problems.push(format!(
                "{path}: the command declares no effects, so an agent must assume it may destroy data"
            ));
        }
        for arg in node.get_arguments() {
            if arg.get_long() != Some("dry-run") {
                continue;
            }
            if arg.get_id() == CRUMBKIT_DRY_RUN {
                if effects.is_some_and(|effects| effects.is_read_only()) {
                    problems.push(format!(
                        "{path}: the command is read-only and takes crumbkit's `--dry-run`, which has nothing to skip"
                    ));
                }
            } else if !allowances.excuses(Rule::DryRun, below) {
                problems.push(format!(
                    "{path}: the command has its own `--dry-run`, and crumbkit's `DryRun` gives that flag one meaning in every tool"
                ));
            }
        }
        let has_yes = node
            .get_arguments()
            .any(|arg| arg.get_long() == Some("yes"));
        asks |= has_yes;
        if effects.is_some_and(|effects| effects.is_destructive())
            && !has_yes
            && !allowances.excuses(Rule::Yes, below)
        {
            problems.push(format!(
                "{path}: the command is destructive and has no `--yes`, so an agent without a terminal cannot confirm it"
            ));
        }
    }
    let has_no_input = command
        .get_arguments()
        .any(|arg| arg.get_id() == CRUMBKIT_NO_INPUT && arg.get_long() == Some("no-input"));
    if asks && !has_no_input && !allowances.excuses(Rule::NoInput, "") {
        problems.push(format!(
            "{}: the tool can ask a question and has no `--no-input` from crumbkit's `NoInput`, so a caller cannot say that nobody can answer",
            command.get_name()
        ));
    }
    allowances.report(command, "effects rules", problems);
}

/// Returns the effects of the first leaf of `guide` whose path is `below`,
/// a command's names below the root joined by single spaces.
fn effects_of(guide: &Guide, below: &str) -> Option<Effects> {
    guide
        .get_leaves()
        .iter()
        .find(|leaf| leaf.path().iter().copied().eq(words(below)))
        .and_then(Leaf::get_effects)
}

/// Reports whether the command at `below` and every command above it are
/// visible, as help shows them.
fn is_visible(root: &Command, below: &str) -> bool {
    let mut command = root;
    for name in words(below) {
        let Some(next) = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == name)
        else {
            return false;
        };
        if next.is_hide_set() {
            return false;
        }
        command = next;
    }
    true
}
