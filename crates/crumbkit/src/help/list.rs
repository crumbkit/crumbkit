//! The headed list of a guided group's subcommands, its compact form of
//! names only, and the line of a list that the goal list and the column
//! list share.

use std::borrow::Cow;
use std::fmt::Write as _;

use clap::Command;
use clap::builder::StyledStr;
use clap::builder::styling::Style;

use super::Goal;

#[cfg(test)]
mod tests;

/// Returns the headed list of the visible subcommands of `node` under
/// `goals`, with `header` as the style of each heading and `literal` as the
/// style of each name, or `None` when it lists no subcommand.
//
// RUST-10: each line holds the name and the description, which ends with
// the command's visible aliases as clap's own list prints them. A tool shows
// them only with an opt-out from crumbkit-test's `check_names` (ADR-0027).
// A description prints without its own styles, because clap
// offers a description's styled text only with its `color` feature. A
// description that clap's `wrap_help` feature wraps, or that
// holds a line break, continues at column 0, where clap's own list indents
// it under the description's first line. A list that indents continuation
// lines as clap does replaces it when a tool needs long descriptions.
pub(super) fn list(
    node: &Command,
    goals: &[Goal],
    header: Style,
    literal: Style,
) -> Option<String> {
    let sections = sections(node, goals);
    let width = sections
        .iter()
        .flat_map(|(_, commands)| commands)
        .map(|command| command.get_name().chars().count())
        .max()?;
    let mut text = String::new();
    for (heading, commands) in sections {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        // Writing to a `String` cannot fail.
        let _ = write!(text, "{header}{heading}{header:#}");
        for command in commands {
            let about = description(command);
            write_line(
                &mut text,
                command.get_name(),
                about.as_deref(),
                width,
                literal,
            );
        }
    }
    Some(text)
}

/// Returns the compact list of the visible subcommands of `node` under
/// `goals`, which a compact group's `-h` prints, or `None` when it lists no
/// subcommand. Each heading that lists a subcommand has one line: the
/// heading and a colon in the `header` style, padding to the longest
/// heading's colon plus two spaces, and the names in the `literal` style,
/// separated by `, `, as in `Read:    list, show`. The headings are the
/// ones that [`list`] prints, in its order, and the line leaves out the
/// descriptions and the aliases.
pub(super) fn names(
    node: &Command,
    goals: &[Goal],
    header: Style,
    literal: Style,
) -> Option<String> {
    let sections = sections(node, goals);
    let width = sections
        .iter()
        .map(|(heading, _)| heading.chars().count())
        .max()?;
    let mut text = String::new();
    for (heading, commands) in sections {
        if !text.is_empty() {
            text.push('\n');
        }
        let padding = width - heading.chars().count() + 2;
        // Writing to a `String` cannot fail.
        let _ = write!(text, "{header}{heading}:{header:#}{:padding$}", "");
        for (index, command) in commands.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }
            let _ = write!(text, "{literal}{}{literal:#}", command.get_name());
        }
    }
    Some(text)
}

/// Returns each heading of `goals` that names a visible subcommand of
/// `node`, with those subcommands in the goal's order, and then the
/// `Commands` heading with the visible subcommands that no goal names, when
/// there are any.
fn sections<'a>(node: &'a Command, goals: &[Goal]) -> Vec<(&'static str, Vec<&'a Command>)> {
    let mut sections: Vec<(&str, Vec<&Command>)> = Vec::new();
    for goal in goals {
        let commands: Vec<&Command> = goal
            .commands
            .iter()
            .filter_map(|&name| {
                node.get_subcommands()
                    .find(|subcommand| subcommand.get_name() == name)
            })
            .filter(|subcommand| !subcommand.is_hide_set())
            .collect();
        if !commands.is_empty() {
            sections.push((goal.heading, commands));
        }
    }
    let unnamed: Vec<&Command> = node
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .filter(|subcommand| {
            !goals
                .iter()
                .any(|goal| goal.commands.contains(&subcommand.get_name()))
        })
        .collect();
    if !unnamed.is_empty() {
        sections.push(("Commands", unnamed));
    }
    sections
}

/// Returns the description of `command` in a goal list: its `about`, or its
/// `long_about` when it has no `about`, and then its visible flag aliases
/// and aliases, as clap's `Commands:` list prints them, such as `List the
/// moons [aliases: ls, l]`, or `[alias: ls]` for one alias. It returns
/// `None` when the command has neither. A command without visible aliases
/// borrows its `about`, so a list without aliases allocates no description.
fn description(command: &Command) -> Option<Cow<'_, StyledStr>> {
    let about = command.get_about().or_else(|| command.get_long_about());
    let visible = command.get_visible_short_flag_aliases().next().is_some()
        || command.get_visible_long_flag_aliases().next().is_some()
        || command.get_visible_aliases().next().is_some();
    if !visible {
        return about.map(Cow::Borrowed);
    }
    let aliases: Vec<String> = command
        .get_visible_short_flag_aliases()
        .map(|alias| format!("-{alias}"))
        .chain(
            command
                .get_visible_long_flag_aliases()
                .map(|alias| format!("--{alias}")),
        )
        .chain(command.get_visible_aliases().map(str::to_owned))
        .collect();
    let mut text = about.cloned().unwrap_or_default();
    if text != StyledStr::new() {
        text.push_str(" ");
    }
    // Writing to a `StyledStr` cannot fail.
    let plural = if aliases.len() == 1 { "" } else { "es" };
    let _ = write!(text, "[alias{plural}: {}]", aliases.join(", "));
    Some(Cow::Owned(text))
}

/// Writes one line of a list to `text`, which the goal list and the topic
/// list share: a line break, two spaces and `name` in the `literal` style,
/// then, when there is an `about`, padding to `width`, the length of the
/// longest name in the list, plus two spaces, and the `about`.
pub(super) fn write_line(
    text: &mut String,
    name: &str,
    about: Option<&StyledStr>,
    width: usize,
    literal: Style,
) {
    // Writing to a `String` cannot fail.
    let _ = write!(text, "\n  {literal}{name}{literal:#}");
    if let Some(about) = about {
        let padding = width - name.chars().count() + 2;
        let _ = write!(text, "{:padding$}{about}", "");
    }
}
