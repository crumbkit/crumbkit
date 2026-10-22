//! The guide check's document rule, which a leaf whose effects change
//! something meets with a document or the declaration that it prints no
//! result (ADR-0016, ADR-0105, ADR-0115, ADR-0143).

use clap::Command;
use crumbkit::help::Leaf;

use crate::allow::{Allowances, Rule};
use crate::tree;

/// Adds a problem to `problems`, for the leaf at `text`, when the leaf
/// declares both a document and that its command prints no result, and
/// when its effects change something, its command takes a view flag, and
/// it declares neither, unless it passes on a program's exit code or an
/// opt-out from [`Rule::GuideDocument`] in `allowances` covers it.
pub(super) fn check_document(
    text: &str,
    leaf: &Leaf,
    command: &Command,
    allowances: &mut Allowances<'_>,
    problems: &mut Vec<String>,
) {
    let path = leaf.path();
    let documented = leaf.get_document().is_some();
    if documented && leaf.get_prints_no_result() {
        problems.push(format!(
            "{text}: the leaf declares a document and also that its command prints no result, so help and parse follow the document and ignore the declaration; keep only `Leaf::document` or `Leaf::stream` when the command prints a result, or only `Leaf::prints_no_result` when it prints none"
        ));
    }
    // A leaf that passes on a program's exit code hands the terminal to
    // it, and `crumbkit_child::hand_over` refuses `--json` and `--fields`
    // before the program starts. A leaf that prints no result offers no
    // view flag, so `parse` refuses each one before the handler runs.
    let changes = leaf
        .get_effects()
        .is_some_and(|effects| !effects.is_read_only());
    let hands_over = leaf.get_passes_on_exit().is_some();
    if changes
        && !hands_over
        && !documented
        && !leaf.get_prints_no_result()
        && takes_view_flag(command, path)
        && !allowances.excuses(Rule::GuideDocument, &path.join(" "))
    {
        problems.push(format!(
            "{text}: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream"
        ));
    }
}

/// Reports whether the command at `path` below `command` takes a view
/// flag: an argument with the id `json`, `fields`, `long` or `full` that
/// is global, which `parse` reads as a view flag. The tree is unbuilt, so
/// the check walks from the leaf's command up to `command` and asks, for
/// each id, the nearest command that declares an argument with it. clap's
/// build copies a global argument down only to a command that has no
/// argument with its id, so that nearest argument is the one the leaf's
/// command parses, and it is a view flag only when it is global.
fn takes_view_flag(command: &Command, path: &[&str]) -> bool {
    let mut chain = vec![command];
    for &name in path {
        let Some(child) = tree::find(chain[chain.len() - 1], [name]) else {
            return false;
        };
        chain.push(child);
    }
    ["json", "fields", "long", "full"].into_iter().any(|id| {
        chain
            .iter()
            .rev()
            .find_map(|node| node.get_arguments().find(|arg| arg.get_id() == id))
            .is_some_and(clap::Arg::is_global_set)
    })
}
