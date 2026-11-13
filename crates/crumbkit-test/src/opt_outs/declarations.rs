//! The declarations section of the opt-out report, which lists the
//! tool's declarations: names and settings that change what a run prints
//! and keep crumbkit's behaviour, so they need no reason (ADR-0056).

use clap::Command;
use crumbkit::help::{Leaf, LeafDocument};
use crumbkit::{Spelling, Tool};
use serde::Serialize;

use crate::allow::{Allow, command_line};
use crate::documents::declared_name;

/// One declaration as the report shows it.
#[derive(Serialize)]
pub(super) struct Declaration {
    /// The kind of declaration, such as `exit-code`.
    #[serde(rename = "declaration")]
    pub(super) kind: &'static str,
    /// What the declaration applies to, such as `every run of shelf` or a
    /// command line.
    pub(super) applies_to: String,
    /// What the declaration changes, in one sentence.
    pub(super) declares: String,
    /// The tool's reason, which only the root verbs declaration has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reason: Option<&'static str>,
}

/// Returns the declarations of `tool` for the tool whose root command is
/// `command`, in this order: each exit that lists one of crumbkit's codes,
/// whether the tool record declares refusal alternatives for every command,
/// whether it declares a refusal hint for every command, the root verbs that
/// [`root_verbs`] lists, and then for each leaf in declared order its
/// document's name when the leaf's path gives another, the program whose exit
/// code it passes on, whether it declares refusal alternatives, whether it
/// declares a refusal hint, the form of its rows' own text line (`Row::LINE`,
/// ADR-0064), and each example that needs a precondition (`Example::needs`,
/// ADR-0163). A declaration that a later change adds joins this list beside
/// the declarations of the same scope: a tool record setting before the
/// leaves, and a leaf's declaration after its refusal-hint entry. The
/// completion variable is not listed, because `complete_with` or
/// `declare_completion_variable` sets it for the process and the tool record
/// does not hold it.
pub(super) fn declarations(command: &Command, tool: &Tool, allow: &[Allow]) -> Vec<Declaration> {
    let every_run = format!("every run of {}", command.get_name());
    let spelling = Spelling::COLONS.for_tool(tool.tool_name(command));
    let mut list = Vec::new();
    for exit in tool.get_failures().get_exits() {
        for &listed in exit.get_failures() {
            if listed.component() != "crumbkit" {
                continue;
            }
            list.push(Declaration {
                kind: "exit-code",
                applies_to: every_run.clone(),
                declares: format!(
                    "A failure with {} exits with {}, where the default is {}.",
                    spelling.spell(listed),
                    exit.code(),
                    Tool::new().failure_exit(listed)
                ),
                reason: None,
            });
        }
    }
    if tool.get_failures().get_refusal_alternatives().is_some() {
        list.push(Declaration {
            kind: "refusal-alternatives",
            applies_to: every_run.clone(),
            declares: "A refused command line for a command without its own function also offers \
                       the tool's own alternatives."
                .to_owned(),
            reason: None,
        });
    }
    if tool.get_failures().get_refusal_hint().is_some() {
        list.push(Declaration {
            kind: "refusal-hint",
            applies_to: every_run.clone(),
            declares: "A refused command line for a command without its own function may recover \
                       with the tool's own hint."
                .to_owned(),
            reason: None,
        });
    }
    list.extend(root_verbs(command, tool, allow));
    for leaf in tool.get_help().get_leaves() {
        list.extend(leaf_declarations(command, leaf));
    }
    list
}

/// Returns the declarations of `leaf` in the tool whose root command is
/// `command`, in the order that [`declarations`] lists them.
fn leaf_declarations(command: &Command, leaf: &Leaf) -> Vec<Declaration> {
    let mut list = Vec::new();
    let printed = leaf.get_document();
    let document = printed.map(LeafDocument::document);
    if let Some((document, name)) = document
        .and_then(|document| Some((document, declared_name(command, leaf.path(), document)?)))
    {
        list.push(Declaration {
            kind: "document-name",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "The command prints {document}, where its path gives the name {name}."
            ),
            reason: None,
        });
    }
    if let Some(program) = leaf.get_passes_on_exit() {
        list.push(Declaration {
            kind: "passes-on-exit",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!("The command exits with the exit code of {program}."),
            reason: None,
        });
    }
    // The report states that the leaf declares refusal alternatives or a
    // refusal hint and leaves out what the functions return, so these
    // getters and the tool record's are the only lines that read them.
    if leaf.get_refusal_alternatives().is_some() {
        list.push(Declaration {
            kind: "refusal-alternatives",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: "A refused command line for this command also offers the tool's own \
                       alternatives."
                .to_owned(),
            reason: None,
        });
    }
    if leaf.get_refusal_hint().is_some() {
        list.push(Declaration {
            kind: "refusal-hint",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: "A refused command line for this command may recover with the tool's own \
                       hint."
                .to_owned(),
            reason: None,
        });
    }
    if let Some(line) = printed.and_then(LeafDocument::line) {
        list.push(Declaration {
            kind: "text-line",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "The text prints each row as the line {line} in place of a table row."
            ),
            reason: None,
        });
    }
    if let Some(field_of) = printed.and_then(LeafDocument::children) {
        let field = field_of.get_field();
        let children = field_of.text();
        // crumbkit's own sentence reads as what the text prints, and a row
        // type's own text is quoted after the field it describes.
        let declares = match field_of.get_help() {
            Some(_) => format!(
                "The text prints lines under each row for its {field}, and help describes them \
                 as: {children}"
            ),
            None => format!("The text prints {children}"),
        };
        list.push(Declaration {
            kind: "children",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares,
            reason: None,
        });
    }
    let text = command_line(command, &leaf.path().join(" "));
    for example in leaf.get_examples() {
        let Some(names) = and_list(example.get_needs()) else {
            continue;
        };
        list.push(Declaration {
            kind: "example-precondition",
            applies_to: crate::help::command_line(&text, example.words()),
            declares: format!("The example runs only in a test run that provides {names}."),
            reason: None,
        });
    }
    list
}

/// Returns a `root-verbs` declaration, with the tool's reason, for the
/// root verbs that `tool` declares and then for those that each
/// [`Allow::root_verbs`] in `allow` declares, which apply to the command
/// line of each verb below `command`.
fn root_verbs(command: &Command, tool: &Tool, allow: &[Allow]) -> Vec<Declaration> {
    let line = |verbs: &[&str]| {
        verbs
            .iter()
            .map(|verb| command_line(command, verb))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut list = Vec::new();
    if let Some((verbs, reason)) = tool.get_help().get_root_verbs() {
        list.push(Declaration {
            kind: "root-verbs",
            applies_to: line(verbs),
            declares:
                "Each command sits directly under the root as a verb, and root help lists it \
                       under the heading Verbs."
                    .to_owned(),
            reason: Some(reason),
        });
    }
    for allow in allow {
        if let Some(verbs) = allow.declared_root_verbs() {
            list.push(Declaration {
                kind: "root-verbs",
                applies_to: line(verbs),
                declares: "Each command sits directly under the root as a verb.".to_owned(),
                reason: Some(allow.reason()),
            });
        }
    }
    list
}

/// Returns `names` as one phrase, such as `server` or `server and reports`
/// or `server, reports and docker`, or `None` when `names` is empty.
fn and_list(names: &[&str]) -> Option<String> {
    let (last, rest) = names.split_last()?;
    if rest.is_empty() {
        return Some((*last).to_owned());
    }
    Some(format!("{} and {last}", rest.join(", ")))
}
