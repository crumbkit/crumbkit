//! The customisations section of the opt-out report, which lists the
//! tool's declarations: names and settings that change what a run prints
//! and keep crumbkit's behaviour, so they need no reason (ADR-0056).

use clap::Command;
use crumbkit::help::{Guide, Leaf, Rename};
use crumbkit::{Code, Spelling};
use serde::Serialize;

use crate::allow::command_line;
use crate::documents::declared_name;
use crate::tree;

/// One declaration as the report shows it.
#[derive(Serialize)]
pub(super) struct Customisation {
    /// The kind of declaration, such as `code-rename`.
    pub(super) declaration: &'static str,
    /// What the declaration applies to, such as `every run of shelf` or a
    /// command line.
    pub(super) applies_to: String,
    /// What the declaration changes, in one sentence.
    pub(super) declares: String,
}

/// Returns the declarations of `guide` for the tool whose root command is
/// `command`, in this order: the failure form, each code rename, the
/// [`Guide::field_names`] declaration, each exit that lists one of
/// crumbkit's codes or the tool's rename of it, whether the guide declares
/// refusal alternatives for every command, the sorted keys that [`Guide::json`] sets
/// (ADR-0067), the [`Guide::dashed_forms`]
/// declaration followed by each command that answers to its dashed form, as
/// [`dashed_names`] lists them, and then for each leaf in guide order its
/// document's name when the leaf's path gives another, the keys that its
/// document's envelope declares (`Envelope::Keys`, ADR-0059), the program
/// whose exit code it passes on, whether it declares refusal alternatives,
/// whether its batch prints its failures in input order
/// (`Envelope::KeysInOrder` or `Envelope::WithoutInOrder`), whether its batch answers each key that
/// the output budget leaves out (`Document::left_out_per_key`), the form of
/// its rows' own text line (`Row::LINE`, ADR-0064), and each code rename of
/// [`Leaf::codes`](crumbkit::help::Leaf::codes). An exit that lists a code
/// that only a leaf renames applies to that leaf's command. A declaration
/// that a later change adds joins this list beside the declarations of the
/// same scope: a guide setting before the leaves, and a leaf's declaration
/// after its refusal-alternatives entry. The completion variable is not
/// listed, because `complete_with` sets it for the process and the guide
/// does not hold it.
pub(super) fn customisations(command: &Command, guide: &Guide) -> Vec<Customisation> {
    let every_run = format!("every run of {}", command.get_name());
    let mut list = Vec::new();
    if let Some(form) = guide.get_failure_form() {
        list.push(Customisation {
            declaration: "failure-form",
            applies_to: every_run.clone(),
            declares: format!(
                "Failures print as {} in place of crumbkit/failure/v1.",
                form.document()
            ),
        });
    }
    for rename in guide.get_codes() {
        list.push(Customisation {
            declaration: "code-rename",
            applies_to: every_run.clone(),
            declares: format!(
                "Runs print {} in place of {}.",
                spell(rename.tool()),
                spell(rename.crumbkit())
            ),
        });
    }
    if let Some((last, earlier)) = guide.get_field_names().split_last() {
        let names = if earlier.is_empty() {
            (*last).to_owned()
        } else {
            format!("{} and {last}", earlier.join(", "))
        };
        list.push(Customisation {
            declaration: "field-names",
            applies_to: every_run.clone(),
            declares: format!(
                "Failures may print the fields {names}, whose names come from another system."
            ),
        });
    }
    for exit in guide.get_exits() {
        for &listed in exit.get_failures() {
            let Some((crumbkit, leaf)) = crumbkit_code(guide, listed) else {
                continue;
            };
            let applies_to = match leaf {
                Some(path) => command_line(command, &path.join(" ")),
                None => every_run.clone(),
            };
            list.push(Customisation {
                declaration: "exit-code",
                applies_to,
                declares: format!(
                    "A failure with {} exits with {}, where crumbkit's default is {}.",
                    spell(listed),
                    exit.code(),
                    Guide::new().failure_exit(&[], crumbkit)
                ),
            });
        }
    }
    if guide.get_refusal_alternatives().is_some() {
        list.push(Customisation {
            declaration: "refusal-alternatives",
            applies_to: every_run.clone(),
            declares: "A refused command line for a command without its own function also offers \
                       the tool's own alternatives."
                .to_owned(),
        });
    }
    if guide.get_json().sorted_keys {
        list.push(Customisation {
            declaration: "json-keys",
            applies_to: every_run.clone(),
            declares: "Every JSON document prints the keys of each object in sorted order."
                .to_owned(),
        });
    }
    if guide.get_dashed_forms() {
        list.push(Customisation {
            declaration: "dashed-forms",
            applies_to: every_run.clone(),
            declares: "A command named in another style also answers to its dashed form."
                .to_owned(),
        });
        list.extend(dashed_names(command));
    }
    for leaf in guide.get_leaves() {
        list.extend(leaf_customisations(command, leaf));
    }
    list
}

/// Returns the declarations of `leaf` in the tool whose root command is
/// `command`, in the order that [`customisations`] lists them.
fn leaf_customisations(command: &Command, leaf: &Leaf) -> Vec<Customisation> {
    let mut list = Vec::new();
    let document = leaf.get_document();
    if let Some((document, name)) = document
        .and_then(|document| Some((document, declared_name(command, leaf.path(), document)?)))
    {
        list.push(Customisation {
            declaration: "document-name",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "The command prints {document}, where its path gives the name {name}."
            ),
        });
    }
    if let Some(document) = document.filter(|document| document.get_envelope().has_own_keys()) {
        let envelope = document.get_envelope();
        list.push(Customisation {
            declaration: "envelope-keys",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "With --json, the command prints {document} under {} and its result under {}, in \
                 place of schema and data.",
                envelope.type_key(),
                envelope.data_key()
            ),
        });
    }
    if let Some(program) = leaf.get_passes_on_exit() {
        list.push(Customisation {
            declaration: "passes-on-exit",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!("The command exits with the exit code of {program}."),
        });
    }
    // The report states that the leaf declares refusal alternatives and
    // leaves out the calls, so this getter and the guide's are the only
    // lines that read them.
    if leaf.get_refusal_alternatives().is_some() {
        list.push(Customisation {
            declaration: "refusal-alternatives",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: "A refused command line for this command also offers the tool's own \
                       alternatives."
                .to_owned(),
        });
    }
    if document.is_some_and(|document| document.get_envelope().failures_in_order()) {
        list.push(Customisation {
            declaration: "failures-in-order",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: "With --json, the command prints its failures among its rows in input \
                       order, in place of text on standard error."
                .to_owned(),
        });
    }
    if document.is_some_and(|document| document.answers_left_out_per_key()) {
        list.push(Customisation {
            declaration: "left-out-per-key",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: "When the output budget leaves out answers, the command prints one failure \
                       for each answer left out, at its key's position, in place of one failure \
                       that names them all. The JSON document can pass the budget by the size \
                       of those failures."
                .to_owned(),
        });
    }
    if let Some(line) = leaf.line() {
        list.push(Customisation {
            declaration: "text-line",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "The text prints each row as the line {line} in place of a table row."
            ),
        });
    }
    for rename in leaf.get_codes() {
        list.push(Customisation {
            declaration: "code-rename",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "Runs print {} in place of {}.",
                spell(rename.tool()),
                spell(rename.crumbkit())
            ),
        });
    }
    list
}

/// Returns a `resource-name` customisation for each command with
/// subcommands, and a `verb-name` customisation for each command without,
/// in the tree of `command`, hidden ones included, whose name is not in
/// crumbkit's dashed form and that answers to its dashed form
/// (`crumbkit::dashed_form`).
fn dashed_names(command: &Command) -> Vec<Customisation> {
    let mut list = Vec::new();
    for visit in tree::walk(command) {
        let Some(parent) = visit.parent() else {
            continue;
        };
        let name = visit.command().get_name();
        if let Some(form) = crumbkit::dashed_form(parent, name) {
            list.push(Customisation {
                declaration: if visit.command().has_subcommands() {
                    "resource-name"
                } else {
                    "verb-name"
                },
                applies_to: visit.path().to_owned(),
                declares: format!("The command also answers to {form}."),
            });
        }
    }
    list
}

/// Returns the code of crumbkit's that `listed` is, or that the guide or
/// one of its leaves renames to `listed`, with the path of the first leaf
/// that renames it when the guide does not, or `None` for one of the
/// tool's own codes.
fn crumbkit_code(
    guide: &Guide,
    listed: &'static Code,
) -> Option<(&'static Code, Option<&'static [&'static str]>)> {
    if listed.component() == "crumbkit" {
        return Some((listed, None));
    }
    let renames_to = |rename: &&Rename| rename.tool() == listed;
    if let Some(rename) = guide.get_codes().iter().find(renames_to) {
        return Some((rename.crumbkit(), None));
    }
    guide.get_leaves().iter().find_map(|leaf| {
        let rename = leaf.get_codes().iter().find(renames_to)?;
        Some((rename.crumbkit(), Some(leaf.path())))
    })
}

/// Returns `code` spelled with colons, as crumbkit's records spell codes.
/// The report does not know the spelling that the tool passes to `finish`.
fn spell(code: &Code) -> String {
    Spelling::COLONS.spell(code)
}
