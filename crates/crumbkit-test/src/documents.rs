//! The document id check and the mount check, which tie each document that
//! a leaf prints to the leaf's path.

use clap::Command;
use crumbkit::help::Guide;

use crate::allow::{Allow, Allowances, Rule};
use crate::schemas::command_line;
use crate::tree;

#[cfg(test)]
mod tests;

/// The commands that crumbkit provides at fixed paths below a tool's root:
/// the name of the document each prints in the component `crumbkit`, and
/// the path the tool mounts it at (ADR-0015). An agent that knows one
/// crumbkit tool runs these commands in every other one by the same words.
pub(crate) const MOUNTS: [(&str, &[&str]); 6] = [
    ("commands-list", &["commands", "list"]),
    ("schemas-list", &["schemas", "list"]),
    ("schemas-show", &["schemas", "show"]),
    ("self-usage", &["self", "usage"]),
    ("self-figures-list", &["self", "figures", "list"]),
    ("aliases-list", &["aliases", "list"]),
];

/// Checks that the name of each document that a leaf of `guide` declares
/// is the leaf's path below the tool joined with `-`, so `shelf books list`
/// prints `shelf/books-list/v1`, and panics, as a failed test assertion
/// does, naming every leaf that breaks the rule (ADR-0007).
///
/// An agent then tells a document's id from the command it ran, and the
/// command from an id that `schemas list` shows. The path of a cargo
/// subcommand starts below its own name: when `command` is named `cargo`,
/// the check drops the first word of each path, so `cargo crumbkit weigh`
/// prints `crumbkit/weigh/v1`. The check reads only the name. A tool chooses
/// the component and the version, and crumbkit's `Document::new` checks
/// their form.
///
/// A tool can opt out of the rule for a command and every command below it
/// with an [`Allow`] of [`Rule::DocumentName`] and a stated reason, such as
/// a document that scripts read under a name from before a rename
/// ([ADR-0036]). The check fails on such an opt-out that names no command
/// or excuses no problem.
///
/// The check also fails on a leaf whose document has no envelope, because
/// the tool built it with `Document::without_envelope`, unless an [`Allow`]
/// of [`Rule::Envelope`] with a stated reason covers the command, so
/// [`check_opt_outs`](crate::check_opt_outs) reports what printing the
/// document unwrapped costs an agent (ADR-0036). A command that prints such
/// a document without a leaf that declares it escapes the check, and review
/// covers it.
///
/// The check also fails on a guide whose failure form, set with
/// `Guide::failure_form`, prints a document in the component `crumbkit`,
/// because crumbkit owns the documents of that component. A failure form
/// is a declaration, which needs no opt-out, and
/// [`check_opt_outs`](crate::check_opt_outs) lists it under the
/// customisations (ADR-0056).
///
/// The check skips a leaf that declares the document of one of crumbkit's
/// commands at fixed paths, such as `crumbkit/schemas-list/v1`, because a
/// tool cannot rename those documents, and [`check_mounts`] checks where
/// the tool mounts them.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::Guide;
///
/// crumbkit_test::check_document_ids(&Command::new("shelf"), &Guide::new(), &[]);
/// ```
///
/// # Panics
///
/// Panics when a document's name or an opt-out breaks a rule above. Each
/// problem in the message starts on a new line and names the leaf's
/// command line, the id, and the name the rule gives.
///
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/adoption/0036-opt-outs.md
pub fn check_document_ids(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::DocumentName, Rule::Envelope]);
    let mut problems = Vec::new();
    if let Some(form) = guide.get_failure_form() {
        let document = form.document();
        if document.component() == "crumbkit" {
            problems.push(format!(
                "{}: the tool's failure form prints {document}, and only crumbkit names documents in the component `crumbkit`, so the form's document needs the tool's own component",
                command.get_name()
            ));
        }
    }
    for leaf in guide.get_leaves() {
        let Some(document) = leaf.get_document() else {
            continue;
        };
        if !document.has_envelope() && !allowances.excuses(Rule::Envelope, &leaf.path().join(" ")) {
            problems.push(format!(
                "{}: the command prints {document} without crumbkit's envelope, which needs an opt-out from `{}` with a reason",
                command_line(command, leaf.path()),
                Rule::Envelope.id()
            ));
        }
        if fixed_path(document.component(), document.name()).is_some() {
            continue;
        }
        let (_, below) = split(command, leaf.path());
        let expected = below.join("-");
        if document.name() != expected
            && !allowances.excuses(Rule::DocumentName, &leaf.path().join(" "))
        {
            problems.push(format!(
                "{}: the command prints {document}, and the name of the document that a command prints is the command's path below the tool joined with dashes, `{expected}`",
                command_line(command, leaf.path())
            ));
        }
    }
    allowances.report(command, "document id rules", problems);
}

/// Checks that each leaf of `guide` that declares the document of one of
/// crumbkit's commands sits at that command's fixed path below the tool,
/// and panics, as a failed test assertion does, naming every leaf at
/// another path (ADR-0015).
///
/// crumbkit's commands are `commands list`, `schemas list`, `schemas show`,
/// `self usage`, `self figures list` and `aliases list`. The check knows a
/// leaf as one of them by the document it declares, such as
/// `crumbkit/schemas-list/v1`. It misses a command that takes crumbkit's
/// arguments type without a leaf that declares the document, because a
/// command tree does not record the Rust type of a command's arguments, so
/// review covers that case. As in [`check_document_ids`], the path of a
/// cargo subcommand starts below its own name.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::Guide;
///
/// crumbkit_test::check_mounts(&Command::new("shelf"), &Guide::new());
/// ```
///
/// # Panics
///
/// Panics when a leaf breaks the rule. Each problem in the message starts
/// on a new line and names the leaf's command line, the document, and the
/// command line the tool mounts the command at.
pub fn check_mounts(command: &Command, guide: &Guide) {
    let mut problems = Vec::new();
    for leaf in guide.get_leaves() {
        let Some(document) = leaf.get_document() else {
            continue;
        };
        let Some(fixed) = fixed_path(document.component(), document.name()) else {
            continue;
        };
        let (above, below) = split(command, leaf.path());
        if below != fixed {
            let expected: Vec<&str> = above.iter().chain(fixed).copied().collect();
            problems.push(format!(
                "{}: the command prints {document}, which crumbkit's command `{}` prints, so the tool mounts it at `{}`",
                command_line(command, leaf.path()),
                fixed.join(" "),
                command_line(command, &expected)
            ));
        }
    }
    tree::report("mount rules", &problems);
}

/// Returns the fixed path of crumbkit's command that prints the document
/// `component/name`, or `None` when no such command prints it.
fn fixed_path(component: &str, name: &str) -> Option<&'static [&'static str]> {
    if component != "crumbkit" {
        return None;
    }
    MOUNTS
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, path)| *path)
}

/// Splits `path` into the words that name the tool and the words below
/// it. The tool is `command` alone, so every word is below it, unless
/// `command` is named `cargo`, when the first word is the cargo
/// subcommand's own name.
fn split<'a>(command: &Command, path: &'a [&'a str]) -> (&'a [&'a str], &'a [&'a str]) {
    if command.get_name() == "cargo" && !path.is_empty() {
        path.split_at(1)
    } else {
        (&[], path)
    }
}
