//! The text of each generated section of a skill file, and the skeleton of
//! a new skill file (ADR-0194).

use std::fmt::Write as _;

use crate::skill::draft::usage::Ranked;
use crate::skill::tree::Tree;

#[cfg(test)]
mod tests;

/// The most flags that the most-used section shows for one command.
const FLAGS: usize = 3;

/// Returns the frontmatter line `description: "<about>"`, from the about
/// text of the tree's root, or from its name when it has none. The value is
/// a JSON string, which YAML reads as a double-quoted string, so a colon or
/// a quote in the about text keeps the frontmatter valid.
pub(crate) fn description(tree: &Tree) -> String {
    let about = tree.root().about.as_deref().unwrap_or(tree.name());
    let quoted = serde_json::to_string(about).unwrap_or_default();
    format!("description: {quoted}\n")
}

/// Returns the job table: one row for each command under each goal heading
/// of the tool's help pages, with the command's about text.
pub(crate) fn jobs(tree: &Tree) -> String {
    let mut text = String::from("## Jobs\n\n");
    let rows: Vec<String> = tree
        .commands()
        .flat_map(|parent| {
            parent.goals.iter().flat_map(move |goal| {
                goal.commands.iter().map(move |name| {
                    let path = format!("{} {name}", parent.path);
                    let about = tree
                        .command(&path)
                        .and_then(|command| command.about.as_deref())
                        .unwrap_or_default();
                    format!("| {} | `{path}` | {} |\n", cell(&goal.heading), cell(about))
                })
            })
        })
        .collect();
    if rows.is_empty() {
        let _ = writeln!(text, "{}'s help declares no goal headings.", tree.name());
    } else {
        text.push_str("| Job | Command | What it does |\n| --- | --- | --- |\n");
        text.extend(rows);
    }
    text
}

/// Returns the most-used section: one line for each of `ranked`, with the
/// command, up to [`FLAGS`] of its most common flags in help notation, and
/// its about text. A flag that the command no longer accepts is left out.
pub(crate) fn most_used(tree: &Tree, ranked: &[Ranked]) -> String {
    let mut text = String::from(
        "## Most-used commands\n\n\
         The commands that callers run most, counting a failed or retried run twice, \
         with their most common flags.\n\n",
    );
    for ranked in ranked {
        let path = format!("{} {}", tree.name(), ranked.command);
        let mut line = path.clone();
        for flag in ranked
            .flags
            .iter()
            .filter_map(|flag| tree.flag(&path, flag))
            .take(FLAGS)
        {
            let name = if flag.name.starts_with("--") {
                &flag.name
            } else {
                flag.short.as_deref().unwrap_or(&flag.name)
            };
            line.push_str(" [");
            line.push_str(name);
            for value in &flag.value_names {
                line.push(' ');
                line.push_str(&value.to_uppercase());
            }
            line.push(']');
        }
        let about = tree
            .command(&path)
            .and_then(|command| command.about.as_deref());
        match about {
            Some(about) => {
                let _ = writeln!(text, "- `{line}`: {about}");
            }
            None => {
                let _ = writeln!(text, "- `{line}`");
            }
        }
    }
    text
}

/// Returns a new skill file for the tree's tool: frontmatter with its name
/// and the `description` section, a heading, a comment where the author
/// writes the hand-written text, and the `jobs` and `usage` sections, each
/// holding the text given for it.
pub(crate) fn skeleton(tree: &Tree, description: &str, jobs: &str, usage: &str) -> String {
    let name = tree.name();
    format!(
        "---\n\
         name: {name}\n\
         # crumbkit:generated description\n\
         {description}\
         # /crumbkit:generated description\n\
         ---\n\
         \n\
         # {name}\n\
         \n\
         <!-- Write here when an agent should use {name} and which command fits which job. \
         cargo crumbkit skill draft rewrites only the sections between crumbkit:generated markers. -->\n\
         \n\
         <!-- crumbkit:generated jobs -->\n\
         {jobs}\
         <!-- /crumbkit:generated jobs -->\n\
         \n\
         <!-- crumbkit:generated usage -->\n\
         {usage}\
         <!-- /crumbkit:generated usage -->\n"
    )
}

/// Returns `text` as one cell of a Markdown table, with each `|` escaped.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}
