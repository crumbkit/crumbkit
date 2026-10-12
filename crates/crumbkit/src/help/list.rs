//! The headed list of a guided group's subcommands, and the line of a list
//! that the goal list, the topic list and the column list share.

use std::fmt::Write as _;

use clap::Command;
use clap::builder::StyledStr;
use clap::builder::styling::Style;

use super::Goal;

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
                about.as_ref(),
                width,
                literal,
            );
        }
    }
    Some(text)
}

/// Returns the description of `command` in a goal list: its `about`, or its
/// `long_about` when it has no `about`, and then its visible flag aliases
/// and aliases, as clap's `Commands:` list prints them, such as `List the
/// moons [aliases: ls, l]`, or `[alias: ls]` for one alias. It returns
/// `None` when the command has neither.
fn description(command: &Command) -> Option<StyledStr> {
    let about = command.get_about().or_else(|| command.get_long_about());
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
    if aliases.is_empty() {
        return about.cloned();
    }
    let mut text = about.cloned().unwrap_or_default();
    if text != StyledStr::new() {
        text.push_str(" ");
    }
    // Writing to a `StyledStr` cannot fail.
    let plural = if aliases.len() == 1 { "" } else { "es" };
    let _ = write!(text, "[alias{plural}: {}]", aliases.join(", "));
    Some(text)
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
