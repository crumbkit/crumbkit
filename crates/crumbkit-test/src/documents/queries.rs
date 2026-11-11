//! The query rule of the mount check: a tool accepts each query that a
//! program which reads it runs (ADR-0202).

use clap::Command;
use crumbkit::queries::{QUERIES, Query};

use crate::schemas::command_line;
use crate::tree;

#[cfg(test)]
mod tests;

/// The id that the check passes to every query. Only a query that takes
/// ids puts it in its words. `schemas show`
/// takes any string as an id when clap parses it, so the check needs no id
/// that the tool prints.
const SAMPLE_ID: &str = "crumbkit/schemas-list/v1";

/// Returns one problem for each query of [`QUERIES`] that `command` mounts
/// and clap refuses. A query whose command the tree does not hold is
/// skipped, so a tree that mounts none of crumbkit's commands has no
/// problem.
///
/// The check parses each query with clap, as the tool's binary does, so it
/// sees the global flags of every command above the query's command and
/// each command that `Command::defer` adds. As in the path rule, a tree
/// named `cargo` holds one tool below each of its subcommands.
pub(super) fn problems(command: &Command) -> Vec<String> {
    let mut built = command.clone();
    built.build();
    let cargo = built.get_name() == "cargo";
    let tools: Vec<Vec<&str>> = if cargo {
        built
            .get_subcommands()
            .map(|tool| vec![tool.get_name()])
            .collect()
    } else {
        vec![Vec::new()]
    };

    let mut problems = Vec::new();
    for above in &tools {
        for query in QUERIES {
            let path: Vec<&str> = above.iter().chain(query.path()).copied().collect();
            let Some(leaf) = tree::find(&built, path.iter().copied()) else {
                continue;
            };
            let words: Vec<&str> = above
                .iter()
                .copied()
                .chain(query.words_with_ids(&[SAMPLE_ID]))
                .collect();
            let line = command_line(command, &words);
            let argv = std::iter::once(command.get_name()).chain(words.iter().copied());
            let Err(error) = command.clone().try_get_matches_from(argv) else {
                continue;
            };
            let missing = missing(leaf, query);
            let reason = if missing.is_empty() {
                format!(
                    "clap refuses it with the error `{:?}`, so make the command accept the query as it is",
                    error.kind()
                )
            } else {
                clauses(&missing)
            };
            problems.push(format!(
                "{}: a program that reads the tool runs `{line}`, and {reason}",
                command_line(command, &path)
            ));
        }
    }
    problems
}

/// Returns the flags of `query` that `leaf`, a command of a built tree,
/// does not take. A built command holds the global flags of every command
/// above it beside its own.
fn missing(leaf: &Command, query: &Query) -> Vec<&'static str> {
    query
        .flags()
        .iter()
        .copied()
        .filter(|flag| {
            !leaf.get_arguments().any(|arg| {
                arg.get_long()
                    .is_some_and(|long| flag.strip_prefix("--") == Some(long))
            })
        })
        .collect()
}

/// Returns the part of a problem that names each flag of `missing` and the
/// change that adds it. Flags that one type adds share a clause, so
/// `--json` and `--full` name `crumbkit::ViewFlags` once.
fn clauses(missing: &[&'static str]) -> String {
    let mut fixes: Vec<(&str, Vec<&str>)> = Vec::new();
    for flag in missing {
        let fix = fix(flag);
        match fixes.iter_mut().find(|(known, _)| *known == fix) {
            Some((_, flags)) => flags.push(flag),
            None => fixes.push((fix, vec![flag])),
        }
    }
    fixes
        .iter()
        .enumerate()
        .map(|(index, (fix, flags))| {
            let subject = if index == 0 { "the command" } else { "it" };
            let flags: Vec<String> = flags.iter().map(|flag| format!("`{flag}`")).collect();
            format!(
                "{subject} does not accept {}, so {fix}",
                flags.join(" and ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Returns the change that makes a command take `flag`, naming the type of
/// crumbkit that adds the flag.
fn fix(flag: &str) -> &'static str {
    match flag {
        "--no-input" => "flatten `crumbkit::NoInput` into the tool's root arguments",
        "--json" | "--full" => "flatten `crumbkit::ViewFlags` into the tool's root arguments",
        "--all" => "flatten `crumbkit::Page` into the command's arguments",
        _ => "add the flag to the command",
    }
}
