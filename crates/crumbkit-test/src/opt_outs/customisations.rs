//! The customisations section of the opt-out report, which lists the
//! tool's declarations: names and settings that change what a run prints
//! and keep crumbkit's behaviour, so they need no reason (ADR-0056).

use clap::Command;
use crumbkit::help::{Guide, Rename};
use crumbkit::{Code, Spelling};
use serde::Serialize;

use crate::allow::command_line;
use crate::documents::declared_name;

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
/// `command`, in this order: the failure form, each code rename, each exit
/// that lists one of crumbkit's codes or the tool's rename of it, and then
/// for each leaf in guide order its document's name when the leaf's path
/// gives another, the program whose exit code it passes on, and whether it
/// declares refusal alternatives. A declaration that a later change adds
/// joins this list beside the declarations of the same scope: a guide
/// setting before the leaves, and a leaf's declaration after its
/// refusal-alternatives entry. The completion variable is not listed,
/// because `complete_with` sets it for the process and the guide does not
/// hold it.
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
    for exit in guide.get_exits() {
        for &listed in exit.get_failures() {
            let Some(crumbkit) = crumbkit_code(guide, listed) else {
                continue;
            };
            list.push(Customisation {
                declaration: "exit-code",
                applies_to: every_run.clone(),
                declares: format!(
                    "A failure with {} exits with {}, where crumbkit's default is {}.",
                    spell(listed),
                    exit.code(),
                    Guide::new().failure_exit(crumbkit)
                ),
            });
        }
    }
    for leaf in guide.get_leaves() {
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
        if let Some(program) = leaf.get_passes_on_exit() {
            list.push(Customisation {
                declaration: "passes-on-exit",
                applies_to: command_line(command, &leaf.path().join(" ")),
                declares: format!("The command exits with the exit code of {program}."),
            });
        }
        // The report states that the leaf declares refusal alternatives and
        // leaves out the calls, so this getter is the only line that reads them.
        if !leaf.get_refusal_alternatives().is_empty() {
            list.push(Customisation {
                declaration: "refusal-alternatives",
                applies_to: command_line(command, &leaf.path().join(" ")),
                declares: "A refused command line for this command also offers the tool's own \
                           alternatives."
                    .to_owned(),
            });
        }
    }
    list
}

/// Returns the code of crumbkit's that `listed` is, or that the guide
/// renames to `listed`, or `None` for one of the tool's own codes.
fn crumbkit_code(guide: &Guide, listed: &'static Code) -> Option<&'static Code> {
    if listed.component() == "crumbkit" {
        return Some(listed);
    }
    guide
        .get_codes()
        .iter()
        .find(|rename| rename.tool() == listed)
        .map(Rename::crumbkit)
}

/// Returns `code` spelled with colons, as crumbkit's records spell codes.
/// The report does not know the spelling that the tool passes to `finish`.
fn spell(code: &Code) -> String {
    Spelling::COLONS.spell(code)
}
