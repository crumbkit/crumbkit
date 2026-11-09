//! The declarations section of the opt-out report, which lists the
//! tool's declarations: names and settings that change what a run prints
//! and keep crumbkit's behaviour, so they need no reason (ADR-0056).

use clap::Command;
use crumbkit::codes::CLI_USAGE;
use crumbkit::help::{Leaf, LeafDocument};
use crumbkit::tool::Rename;
use crumbkit::{Code, Spelling, Tool};
use serde::Serialize;

use crate::allow::{Allow, command_line};
use crate::documents::declared_name;
use crate::tree;

/// One declaration as the report shows it.
#[derive(Serialize)]
pub(super) struct Declaration {
    /// The kind of declaration, such as `code-rename`.
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
/// `command`, in this order: the tool record's spelling when it spells
/// crumbkit's usage code other than [`Spelling::COLONS`] does, each code
/// rename, each exit that lists one of crumbkit's codes or the tool's rename of
/// it, whether the tool record declares refusal alternatives for every command,
/// whether it declares a refusal hint for every command, the
/// [`Tool::dashed_forms`] declaration followed by each command that answers to
/// its dashed form, as [`dashed_names`] lists them, the root verbs that
/// [`root_verbs`] lists, and then for each leaf in declared order its
/// document's name when the leaf's path gives another, the program whose exit
/// code it passes on, whether it declares refusal alternatives, whether it
/// declares a refusal hint, the form of its rows' own text line (`Row::LINE`,
/// ADR-0064), each code rename of [`Leaf::codes`](crumbkit::help::Leaf::codes),
/// and each example that needs a precondition (`Example::needs`, ADR-0163). An
/// exit that lists a code that only a leaf renames applies to that leaf's
/// command. A declaration that a later change adds joins this list beside the
/// declarations of the same scope: a tool record setting before the leaves, and
/// a leaf's declaration after its refusal-hint entry. The completion variable
/// is not listed, because `complete_with` sets it for the process and the tool
/// record does not hold it.
pub(super) fn declarations(command: &Command, tool: &Tool, allow: &[Allow]) -> Vec<Declaration> {
    let every_run = format!("every run of {}", command.get_name());
    let tool_name = tool.tool_name(command);
    let spelling = tool.get_failures().get_spelling().for_tool(tool_name);
    let spell = |code: &Code| spelling.spell(code);
    let mut list = Vec::new();
    let colons = Spelling::COLONS.for_tool(tool_name).spell(&CLI_USAGE);
    let usage = spell(&CLI_USAGE);
    if usage != colons {
        list.push(Declaration {
            kind: "code-spelling",
            applies_to: every_run.clone(),
            declares: format!("Runs print codes such as {usage} in place of {colons}."),
            reason: None,
        });
    }
    for rename in tool.get_failures().get_codes() {
        list.push(Declaration {
            kind: "code-rename",
            applies_to: every_run.clone(),
            declares: format!(
                "Runs print {} in place of {}.",
                spell(rename.tool()),
                spell(rename.crumbkit())
            ),
            reason: None,
        });
    }
    for exit in tool.get_failures().get_exits() {
        for &listed in exit.get_failures() {
            let Some((crumbkit, leaf)) = crumbkit_code(tool, listed) else {
                continue;
            };
            let applies_to = match leaf {
                Some(path) => command_line(command, &path.join(" ")),
                None => every_run.clone(),
            };
            list.push(Declaration {
                kind: "exit-code",
                applies_to,
                declares: format!(
                    "A failure with {} exits with {}, where the default is {}.",
                    spell(listed),
                    exit.code(),
                    Tool::new().failure_exit(&[], crumbkit)
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
    if tool.get_dashed_forms() {
        list.push(Declaration {
            kind: "dashed-forms",
            applies_to: every_run.clone(),
            declares: "A command named in another style also answers to its dashed form."
                .to_owned(),
            reason: None,
        });
        list.extend(dashed_names(command));
    }
    list.extend(root_verbs(command, tool, allow));
    for leaf in tool.get_help().get_leaves() {
        list.extend(leaf_declarations(command, spelling, leaf));
    }
    list
}

/// Returns the declarations of `leaf` in the tool whose root command is
/// `command`, in the order that [`declarations`] lists them, with each
/// code in `spelling`.
fn leaf_declarations(command: &Command, spelling: Spelling, leaf: &Leaf) -> Vec<Declaration> {
    let spell = |code: &Code| spelling.spell(code);
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
    for rename in leaf.get_codes() {
        list.push(Declaration {
            kind: "code-rename",
            applies_to: command_line(command, &leaf.path().join(" ")),
            declares: format!(
                "Runs print {} in place of {}.",
                spell(rename.tool()),
                spell(rename.crumbkit())
            ),
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

/// Returns a `resource-name` declaration for each command with
/// subcommands, and a `verb-name` declaration for each command without,
/// in the tree of `command`, hidden ones included, whose name is not in
/// crumbkit's dashed form and that answers to its dashed form
/// (`crumbkit::dashed_form`).
fn dashed_names(command: &Command) -> Vec<Declaration> {
    let mut list = Vec::new();
    for visit in tree::walk(command) {
        let Some(parent) = visit.parent() else {
            continue;
        };
        let name = visit.command().get_name();
        if let Some(form) = crumbkit::dashed_form(parent, name) {
            list.push(Declaration {
                kind: if visit.command().has_subcommands() {
                    "resource-name"
                } else {
                    "verb-name"
                },
                applies_to: visit.path().to_owned(),
                declares: format!("The command also answers to {form}."),
                reason: None,
            });
        }
    }
    list
}

/// Returns the code of crumbkit's that `listed` is, or that the tool record or
/// one of its leaves renames to `listed`, with the path of the first leaf
/// that renames it when the tool record does not, or `None` for one of the
/// tool's own codes.
fn crumbkit_code(
    tool: &Tool,
    listed: &'static Code,
) -> Option<(&'static Code, Option<&'static [&'static str]>)> {
    if listed.component() == "crumbkit" {
        return Some((listed, None));
    }
    let renames_to = |rename: &&Rename| rename.tool() == listed;
    if let Some(rename) = tool.get_failures().get_codes().iter().find(renames_to) {
        return Some((rename.crumbkit(), None));
    }
    tool.get_help().get_leaves().iter().find_map(|leaf| {
        let rename = leaf.get_codes().iter().find(renames_to)?;
        Some((rename.crumbkit(), Some(leaf.path())))
    })
}
