//! The document id check and the mount check, which tie each document that
//! a leaf prints to the leaf's path. The mount check also parses the queries
//! that a program which reads a tool runs.

use clap::Command;
use crumbkit::help::LeafDocument;
use crumbkit::{Document, Tool};

use crate::allow::{Allow, Allowances, Rule};
use crate::schemas::command_line;
use crate::tree;

mod queries;
#[cfg(test)]
mod tests;

/// The commands that crumbkit provides at fixed paths below a tool's root:
/// the name of the document each prints in the component `crumbkit`, and
/// the path the tool mounts it at (ADR-0015). An agent that knows one
/// crumbkit tool runs these commands in every other one by the same words.
pub(crate) const MOUNTS: [(&str, &[&str]); 8] = [
    ("commands-list", &["commands", "list"]),
    ("schemas-list", &["schemas", "list"]),
    ("schemas-show", &["schemas", "show"]),
    ("self-usage", &["self", "usage"]),
    ("self-figures-list", &["self", "figures", "list"]),
    ("self-runs-list", &["self", "runs", "list"]),
    ("self-runs-export", &["self", "runs", "export"]),
    ("aliases-list", &["aliases", "list"]),
];

/// Checks the documents that the leaves of `tool` declare, and panics, as a
/// failed test assertion does, naming every problem it finds.
///
/// The check fails on a leaf whose document has no envelope, because
/// the tool chose [`Envelope::Without`](crumbkit::Envelope::Without),
/// unless an [`Allow`] of [`Rule::Envelope`] with a stated reason covers the
/// command, so [`check_opt_outs`](crate::check_opt_outs) reports what
/// printing the document unwrapped costs an agent ([opt-outs docs]). A
/// command that prints such a document without a leaf that declares it
/// escapes the check, and review covers it.
///
/// The check fails on a leaf whose row type sets `Row::WHOLE`, so `--json`
/// prints every field of each row without `--full`, unless an [`Allow`] of
/// [`Rule::RowsWhole`] with a stated reason covers the command, so
/// [`check_opt_outs`](crate::check_opt_outs) reports that the JSON costs
/// more tokens than the compact columns (ADR-0065).
///
/// A document's name is the tool's own name, so a document whose name is
/// not the leaf's path below the tool joined with `-`, as
/// `shelf/books-list/v1` is for `shelf books list`, is a declaration. The
/// check accepts it without an opt-out, and
/// [`check_opt_outs`](crate::check_opt_outs) lists it under the
/// declarations (ADR-0056).
///
/// ```
/// use clap::Command;
/// use crumbkit::Tool;
///
/// crumbkit_test::check_document_ids(&Command::new("shelf"), &Tool::new(), &[]);
/// ```
///
/// # Panics
///
/// Panics when a document or an opt-out breaks a rule above. Each
/// problem in the message starts on a new line and names the leaf's
/// command line, the id, and the name the rule gives.
///
/// [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
pub fn check_document_ids(command: &Command, tool: &Tool, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::Envelope, Rule::RowsWhole]);
    let mut problems = Vec::new();
    for leaf in tool.get_help().get_leaves() {
        let Some(declared) = leaf.get_document() else {
            continue;
        };
        let document = declared.document();
        if !document.get_envelope().wraps()
            && !allowances.excuses(Rule::Envelope, &leaf.path().join(" "))
        {
            problems.push(format!(
                "{}: the command prints {document} without crumbkit's envelope, which needs an opt-out from `{}` with a reason",
                command_line(command, leaf.path()),
                Rule::Envelope.id()
            ));
        }
        if declared.rows_whole() && !allowances.excuses(Rule::RowsWhole, &leaf.path().join(" ")) {
            problems.push(format!(
                "{}: the command prints the rows of {document} whole, because its row type sets `Row::WHOLE`, which needs an opt-out from `{}` with a reason",
                command_line(command, leaf.path()),
                Rule::RowsWhole.id()
            ));
        }
    }
    allowances.report(command, "document id rules", problems);
}

/// Returns the name that the path of the leaf at `path` gives its document
/// when the document's name is another, which the tool then declares, or
/// `None` when the name is the path's or `document` is one of crumbkit's
/// documents at fixed paths, which [`check_mounts`] checks.
///
/// The path's name is the path below the tool joined with `-`, so `shelf
/// books list` gives `books-list`, and an agent tells a document's id from
/// the command it ran (ADR-0035). The path of a cargo subcommand starts
/// below its own name: when `command` is named `cargo`, the first word of
/// the path is dropped, so `cargo crumbkit weigh` gives `weigh`.
pub(crate) fn declared_name(
    command: &Command,
    path: &[&str],
    document: Document<()>,
) -> Option<String> {
    if fixed_path(document.component(), document.name()).is_some() {
        return None;
    }
    let (_, below) = split(command, path);
    let expected = below.join("-");
    (document.name() != expected).then_some(expected)
}

/// Checks that each leaf of `tool` that declares the document of one of
/// crumbkit's commands sits at that command's fixed path below the tool,
/// and panics, as a failed test assertion does, naming every leaf at
/// another path (ADR-0015).
///
/// crumbkit's commands are `commands list`, `schemas list`, `schemas show`,
/// `self usage`, `self figures list`, `self runs list`, `self runs export`
/// and `aliases list`. The check knows a leaf as one of them by the document
/// it declares, such as `<tool>/schemas-list/v1`. It misses a command that takes crumbkit's
/// arguments type without a leaf that declares the document, because a
/// command tree does not record the Rust type of a command's arguments, so
/// review covers that case. As in [`check_document_ids`], the path of a
/// cargo subcommand starts below its own name.
///
/// The check also parses each query of [`crumbkit::queries::QUERIES`] whose
/// command `command` holds, with clap, as the tool's binary would. They are
/// the command lines that a program which reads a tool runs, such as
/// `commands list --json --full --all --no-input`. The check fails on a
/// query that clap refuses and names each flag the command does not take
/// with the type that adds it: [`crumbkit::NoInput`] for `--no-input`,
/// [`crumbkit::ViewFlags`] for `--json` and `--full`, and [`crumbkit::Page`]
/// for `--all`. A query whose command the tree does not hold is skipped, so
/// a tool that mounts none of the commands passes. The rule has no opt-out
/// (ADR-0202).
///
/// ```
/// use clap::Command;
/// use crumbkit::Tool;
///
/// crumbkit_test::check_mounts(&Command::new("shelf"), &Tool::new());
/// ```
///
/// # Panics
///
/// Panics when a leaf breaks the path rule or clap refuses a query. Each
/// problem in the message starts on a new line. A problem of the path rule
/// names the leaf's command line, the document, and the command line the
/// tool mounts the command at. A problem of a query names the command, the
/// whole query, and each flag to add with the type that adds it.
pub fn check_mounts(command: &Command, tool: &Tool) {
    let mut problems = Vec::new();
    for leaf in tool.get_help().get_leaves() {
        let Some(document) = leaf.get_document().map(LeafDocument::document) else {
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
    problems.extend(queries::problems(command));
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
