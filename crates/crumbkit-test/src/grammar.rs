//! The grammar check, which gives every command the form
//! `<tool> <resource> <verb>`.

use clap::Command;
use crumbkit::help::Guide;

use crate::Pages;
use crate::allow::{Allow, Allowances, Rule};
use crate::tree::{self, DASHED_NAME};

#[cfg(test)]
mod tests;

/// The rules of the grammar check that a tool can opt out of.
const RULES: [Rule; 2] = [Rule::RootLeaf, Rule::ResourceName];

/// The verbs that every tool has, which a tool does not pass to
/// [`check_grammar`].
const BUILT_IN_VERBS: [&str; 2] = ["list", "show"];

/// Checks that every command in the tree of `command` is a resource or a
/// verb, and panics, as a failed test assertion does, naming every problem it
/// finds.
///
/// `command` is the tool, or the root of a subtree, and is checked as neither
/// a resource nor a verb. `verbs` holds the tool's verbs beside `list` and
/// `show`. The check walks `command` and every visible subcommand below it,
/// because the grammar is what help, `commands list` and the command
/// reference show an agent, and they all leave out each hidden command and
/// everything below it. A hidden command, such as an internal `__query`,
/// is therefore no resource and no verb. Only the rule that reserves `help`
/// also covers hidden commands, because a hidden command named `help` still
/// takes that word from crumbkit's `parse`. It finds:
///
/// - a command without subcommands, which is a verb, whose name is not
///   `list`, `show` or one of `verbs`;
/// - a command without subcommands directly under `command`, because every
///   command has the form `<tool> <resource> <verb>`, unless the tool
///   declares it as a root verb with `Guide::root_verbs` or
///   [`Allow::root_verbs`], or an opt-out from [`Rule::RootLeaf`] in `allow`
///   names it, such as `version`. The verb rule above skips such a leaf;
/// - a resource, at any depth, whose name or alias is the name or an alias
///   of a declared root verb, so a resource that the tool adds later cannot
///   take a root verb's word. This rule has no opt-out;
/// - a declared root verb that names no visible command without
///   subcommands directly under `command`, a declaration without a verb, a
///   verb that one declaration names twice, and root verbs
///   declared more than once, on the guide and with
///   `Allow::root_verbs` or with `Allow::root_verbs` twice, because a tool
///   declares its root verbs once with one reason;
/// - a command with subcommands, which is a resource, whose name is not
///   lowercase ASCII letters and digits in words joined by single dashes,
///   starting with a letter, such as `reading-lists`, unless an opt-out
///   from [`Rule::ResourceName`] covers it. When the guide declares
///   `Guide::dashed_forms`, a resource named in another style, such as
///   `reading_lists` or `ReadingLists`, passes, because crumbkit's `parse`
///   and completion then read its dashed form as the resource
///   (`crumbkit::dashed_form`). It still fails when its name maps to no
///   dashed form, such as `2books`, when the form is `help`, or when
///   another command beside it has the form as its name or alias or maps
///   to it too. Resources may nest;
/// - an entry of `verbs` that is `list` or `show`, or whose name is not in
///   the dashed form and, with `Guide::dashed_forms`, maps to none;
/// - with `Guide::dashed_forms`, a command without subcommands below
///   `command` whose name maps to a dashed form that it does not answer to,
///   for the reasons above. This rule has no opt-out, because a tool
///   declares its verbs;
/// - a command below `command`, hidden or visible, named `help` or with the
///   alias `help`, because crumbkit reserves that word for asking for
///   help. crumbkit's `parse` answers it below every command with
///   subcommands, unless such a command takes it, so this rule has no
///   opt-out;
/// - an opt-out from [`Rule::RootLeaf`] or [`Rule::ResourceName`] that
///   names no command or excuses no problem.
///
/// An opt-out covers the command it names and every command below it, and
/// the check reads only the opt-outs of its own rules from `allow`.
///
/// `pages` gives the check the tool's guide: [`Pages::rendered`] and
/// [`Pages::binary`] carry it, and [`Pages::clap`], for a tool that parses
/// with clap alone, carries none, so `Guide::dashed_forms` is off and only
/// [`Allow::root_verbs`] declares root verbs. The check reads no help page.
///
/// [`check_effects`](crate::check_effects) checks `--dry-run`.
///
/// ```
/// use clap::Command;
/// use crumbkit_test::{Allow, Pages, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(
///     Rule::RootLeaf,
///     "version",
///     "People type `shelf version` from habit.",
/// )];
///
/// let shelf = Command::new("shelf")
///     .subcommand(
///         Command::new("books")
///             .subcommand(Command::new("list"))
///             .subcommand(Command::new("export")),
///     )
///     .subcommand(Command::new("version"));
///
/// crumbkit_test::check_grammar(&shelf, Pages::clap(), &["export"], ALLOW);
/// ```
///
/// # Panics
///
/// Panics when the tree breaks a rule above. The message lists each problem
/// on its own line and names each command by its path from `command`, such
/// as `shelf books list`.
pub fn check_grammar(command: &Command, pages: Pages<'_>, verbs: &[&str], allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &RULES);
    let dashed_forms = pages.guide().is_some_and(Guide::get_dashed_forms);
    let mut problems = Vec::new();
    for &verb in verbs {
        if BUILT_IN_VERBS.contains(&verb) {
            problems.push(format!(
                "the tool passes the verb `{verb}`, which is built in, and a tool passes only its other verbs"
            ));
        } else if !crumbkit::is_dashed(verb) && !dashed_forms {
            problems.push(format!("the verb `{verb}` is not {DASHED_NAME}"));
        } else if !crumbkit::is_dashed(verb) && crumbkit::dashed_name(verb).is_none() {
            problems.push(format!(
                "the verb `{verb}` is not {DASHED_NAME}, and maps to no such name"
            ));
        }
    }
    let tool = command.get_name();
    let root_verbs = declared_root_verbs(pages.guide(), allow, &mut problems);
    let root_words = root_words(command, &root_verbs);
    for visit in tree::walk(command) {
        let path = visit.path();
        let below = visit.below();
        let command = visit.command();
        let name = command.get_name();
        let in_grammar = visit.is_visible();
        match (visit.depth(), command.has_subcommands()) {
            // The command passed in is the tool or a subtree's root, which
            // is neither a resource nor a verb, and a hidden command and
            // everything below it are outside the grammar.
            (0, _) => {}
            _ if !in_grammar => {}
            (_, true) => {
                if let Some(problem) = name_problem(visit.parent(), dashed_forms, name, "resource")
                    && !allowances.excuses(Rule::ResourceName, below)
                {
                    problems.push(format!("{path}: {problem}"));
                }
                let words = std::iter::once(name).chain(command.get_all_aliases());
                for word in words {
                    if let Some((verb, _)) = root_words.iter().find(|(_, taken)| *taken == word) {
                        problems.push(format!(
                            "{path}: the resource answers to `{word}`, which the root verb `{verb}` also answers to"
                        ));
                    }
                }
            }
            (1, false) => {
                if !root_verbs.contains(&name) && !allowances.excuses(Rule::RootLeaf, below) {
                    problems.push(format!(
                        "{path}: the command has no subcommands and sits directly under `{tool}`, and a command has the form `{tool} <resource> <verb>`"
                    ));
                }
            }
            (_, false) => {
                if !BUILT_IN_VERBS.contains(&name) && !verbs.contains(&name) {
                    problems.push(format!(
                        "{path}: the command has no subcommands, so it is a verb, but `{name}` is not `list`, `show` or a verb the tool passes"
                    ));
                }
            }
        }
        // The verb rules above check the form of each verb's name, so this
        // check reports only a leaf whose dashed form another command takes.
        if dashed_forms
            && in_grammar
            && visit.depth() > 0
            && !command.has_subcommands()
            && crumbkit::dashed_name(name).is_some()
            && let Some(problem) = name_problem(visit.parent(), true, name, "verb")
        {
            problems.push(format!("{path}: {problem}"));
        }
        if visit.depth() > 0 && (name == "help" || command.get_all_aliases().any(|a| a == "help")) {
            problems.push(format!(
                "{path}: the command answers to `help`, and crumbkit reserves that word for asking for help"
            ));
        }
    }
    for verb in root_verbs {
        if !root_words.iter().any(|&(declared, _)| declared == verb) {
            problems.push(format!(
                "the root verb `{verb}` names no visible command without subcommands directly under `{tool}`, so its declaration is stale"
            ));
        }
    }
    allowances.report(command, "grammar", problems);
}

/// Returns the root verbs that the tool declares on `guide` and with
/// `Allow::root_verbs` in `allow`, and adds a problem to `problems` when it
/// declares them more than once, declares a list without a verb, or names
/// one verb twice in one declaration.
///
/// A tool declares its root verbs in one place, because root help lists
/// only the guide's under their heading, and one reason then covers the
/// whole list. The check still reads every declaration, so it reports the
/// other problems of each.
fn declared_root_verbs(
    guide: Option<&Guide>,
    allow: &[Allow],
    problems: &mut Vec<String>,
) -> Vec<&'static str> {
    let declarations: Vec<&[&str]> = guide
        .and_then(Guide::get_root_verbs)
        .map(|(verbs, _)| verbs)
        .into_iter()
        .chain(allow.iter().filter_map(Allow::declared_root_verbs))
        .collect();
    if declarations.len() > 1 {
        problems.push(
            "the tool declares root verbs more than once, and a tool declares them once, on its guide or with `Allow::root_verbs`"
                .to_owned(),
        );
    }
    if declarations.iter().any(|verbs| verbs.is_empty()) {
        problems.push(
            "the tool declares root verbs without naming one, so the declaration is stale"
                .to_owned(),
        );
    }
    for verbs in &declarations {
        for (index, verb) in verbs.iter().enumerate() {
            if verbs[..index].contains(verb) && !verbs[index + 1..].contains(verb) {
                problems.push(format!(
                    "the root verb `{verb}` is declared more than once, so root help would list it twice"
                ));
            }
        }
    }
    declarations.into_iter().flatten().copied().collect()
}

/// Returns each word that a root verb in `verbs` answers to, its name and
/// each alias, beside the verb's name. A verb answers to words only when it
/// names a visible command without subcommands directly under `command`.
fn root_words<'a>(command: &'a Command, verbs: &[&'static str]) -> Vec<(&'static str, &'a str)> {
    let mut words = Vec::new();
    for &verb in verbs {
        let leaf = command.get_subcommands().find(|subcommand| {
            subcommand.get_name() == verb
                && !subcommand.is_hide_set()
                && !subcommand.has_subcommands()
        });
        if let Some(leaf) = leaf {
            words.push((verb, leaf.get_name()));
            words.extend(leaf.get_all_aliases().map(|alias| (verb, alias)));
        }
    }
    words
}

/// Returns the problem with `name`, the name of a `kind` of command that
/// `parent` holds, or `None` when the name is lowercase dashed words or,
/// with `dashed_forms`, the command answers to its dashed form, as
/// `crumbkit::dashed_form` finds it. With `dashed_forms`, the problem says
/// whether the name maps to no dashed form, maps to `help`, or maps to a
/// form that another command beside it has or maps to.
fn name_problem(
    parent: Option<&Command>,
    dashed_forms: bool,
    name: &str,
    kind: &str,
) -> Option<String> {
    if crumbkit::is_dashed(name) {
        return None;
    }
    if !dashed_forms {
        return Some(format!("the {kind} name `{name}` is not {DASHED_NAME}"));
    }
    let Some(form) = crumbkit::dashed_name(name) else {
        return Some(format!(
            "the {kind} name `{name}` is not {DASHED_NAME}, and maps to no such name"
        ));
    };
    if parent.is_some_and(|parent| crumbkit::dashed_form(parent, name).is_some()) {
        return None;
    }
    Some(if form == "help" {
        format!(
            "the {kind} name `{name}` maps to `help`, which crumbkit reserves for asking for help"
        )
    } else {
        format!(
            "the {kind} name `{name}` maps to `{form}`, which another command beside it has or maps to"
        )
    })
}
