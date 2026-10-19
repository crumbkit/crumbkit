//! The effects check, which requires every leaf to declare its effects and
//! gives `--dry-run`, `--yes` and `--no-input` one meaning in every tool.

use clap::{Arg, Command};
use crumbkit::Shape;
use crumbkit::help::{Effects, Guide, Items, Leaf};

use crate::allow::{Allow, Allowances, Rule, words};
use crate::filters::accepts_many;
use crate::tree;

#[cfg(test)]
mod tests;

/// The rules of the effects check that a tool can opt out of.
const RULES: [Rule; 7] = [
    Rule::Effects,
    Rule::DryRun,
    Rule::Yes,
    Rule::NoInput,
    Rule::Expect,
    Rule::ExpectFlag,
    Rule::Items,
];

/// The id that `crumbkit::DryRun` gives its `--dry-run`, which tells it
/// apart from a tool's own flag with the same name.
const CRUMBKIT_DRY_RUN: &str = "crumbkit_dry_run";

/// The id that `crumbkit::NoInput` gives its `--no-input`.
const CRUMBKIT_NO_INPUT: &str = "crumbkit_no_input";

/// The id that `crumbkit::Expect` gives its `--expect`.
const CRUMBKIT_EXPECT: &str = "crumbkit_expect";

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
/// - a leaf whose effects are [`Effects::replacing`] without the `--expect`
///   of `crumbkit::Expect`, unless an opt-out from [`Rule::Expect`] covers
///   it, because two writers can then lose each other's changes;
/// - an argument anywhere in the tree with the long name `--expect` that
///   `crumbkit::Expect` did not add, unless an opt-out from
///   [`Rule::ExpectFlag`] covers its command;
/// - crumbkit's `--expect` on a read-only leaf, which has nothing to guard.
///   This rule has no opt-out;
/// - a leaf that writes, takes many values in a visible positional
///   argument, and declares neither [`Effects::all_or_none`] nor
///   [`Effects::per_item`], unless an opt-out from [`Rule::Items`] covers
///   it, because an agent cannot tell what a failed run changed;
/// - a leaf that declares [`Effects::per_item`] and whose document is not a
///   `Batch`, a `Prose` or a stream, which are the documents that report a
///   failure for each item. This rule has no opt-out;
/// - an opt-out from one of the seven rules above that names no command or
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
/// [`Effects::replacing`]: crumbkit::help::Effects::replacing
/// [`Effects::all_or_none`]: crumbkit::help::Effects::all_or_none
/// [`Effects::per_item`]: crumbkit::help::Effects::per_item
pub fn check_effects(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &RULES);
    let mut problems = Vec::new();
    let mut asks = guide.get_picker().is_some();
    for visit in tree::walk(command) {
        let path = visit.path();
        let below = visit.below();
        let node = visit.command();
        let leaf = if node.has_subcommands() {
            None
        } else {
            leaf_of(guide, below)
        };
        let effects = leaf.and_then(Leaf::get_effects);
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
        check_expect(node, path, effects, &mut allowances, &mut problems);
        if let Some(leaf) = leaf {
            check_items(node, path, leaf, &mut allowances, &mut problems);
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

/// Returns the first leaf of `guide` whose path is `below`, a command's
/// names below the root joined by single spaces.
fn leaf_of<'a>(guide: &'a Guide, below: &str) -> Option<&'a Leaf> {
    guide
        .get_leaves()
        .iter()
        .find(|leaf| leaf.path().iter().copied().eq(words(below)))
}

/// Adds to `problems` each `--expect` rule that `node`, at `path`, breaks
/// with `effects`, the effects of its leaf or `None` for a branch or a
/// leaf without effects.
fn check_expect(
    node: &Command,
    path: &str,
    effects: Option<Effects>,
    allowances: &mut Allowances<'_>,
    problems: &mut Vec<String>,
) {
    let below = below_of(path);
    let mut takes_expect = false;
    for arg in node.get_arguments() {
        if arg.get_long() != Some("expect") {
            continue;
        }
        if arg.get_id() == CRUMBKIT_EXPECT {
            takes_expect = true;
            if effects.is_some_and(|effects| effects.is_read_only()) {
                problems.push(format!(
                    "{path}: the command is read-only and takes crumbkit's `--expect`, which has nothing to guard"
                ));
            }
        } else if !allowances.excuses(Rule::ExpectFlag, below) {
            problems.push(format!(
                "{path}: the command has its own `--expect`, and crumbkit's `Expect` gives that flag one meaning in every tool"
            ));
        }
    }
    if effects.is_some_and(|effects| effects.is_replacing())
        && !takes_expect
        && !allowances.excuses(Rule::Expect, below)
    {
        problems.push(format!(
            "{path}: the command replaces existing data and has no `--expect` from crumbkit's `Expect`, so two writers can lose each other's changes"
        ));
    }
}

/// Adds to `problems` each rule about many items that `node`, at `path`,
/// breaks with `leaf`, the leaf of `guide` for the command.
fn check_items(
    node: &Command,
    path: &str,
    leaf: &Leaf,
    allowances: &mut Allowances<'_>,
    problems: &mut Vec<String>,
) {
    let Some(effects) = leaf.get_effects() else {
        return;
    };
    match effects.items() {
        Some(Items::PerItem) => {
            let reports =
                leaf.is_stream() || matches!(leaf.shape(), Some(Shape::Batch | Shape::Prose));
            if !reports {
                problems.push(format!(
                    "{path}: the command declares `per_item` and its document is not a `Batch`, a `Prose` or a stream, so it cannot report a failure for each item"
                ));
            }
        }
        Some(_) => {}
        // A read-only leaf writes no items, and a `const` that calls
        // `Effects::all_or_none` or `Effects::per_item` on
        // `Effects::read_only` fails to compile.
        None if effects.is_read_only() => {}
        None => {
            // The check reads a visible positional argument that takes many
            // values as the items that a write changes, such as the ids of
            // `books remove`. An option with many values more often filters
            // or describes the change, such as `--tag`, so the check leaves
            // options alone, and a tool whose positional values are not the
            // items it writes opts out.
            if let Some(items) = node.get_arguments().find(|arg| names_many_items(arg))
                && !allowances.excuses(Rule::Items, below_of(path))
            {
                problems.push(format!(
                    "{path}: the command writes the many items of `{}` and declares neither `all_or_none` nor `per_item`, so an agent cannot tell what a failed run changed",
                    items.get_id()
                ));
            }
        }
    }
}

/// Reports whether `arg` is a visible positional argument that takes many
/// values.
fn names_many_items(arg: &Arg) -> bool {
    arg.is_positional() && !arg.is_hide_set() && accepts_many(arg)
}

/// Returns the names below the root in `path`, such as `books remove` for
/// `shelf books remove`, as an opt-out names the command.
fn below_of(path: &str) -> &str {
    path.split_once(' ').map_or("", |(_, below)| below)
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
