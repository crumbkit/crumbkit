//! The one-line message of a command line that clap refused.

use clap::Command;
use clap::error::ErrorKind;

use super::lines::visible_subcommands;
use super::suggestion;

/// Returns the message of a refusal of a command line that ends at the
/// command at `path`.
///
/// A command line that stops at a command that needs more, which clap
/// answers with that command's help page or a list of its subcommands, says
/// what the command needs. A group's message names its visible
/// subcommands in the order that the tool declares them, as in
/// ``the command `tool planets` needs a subcommand: show, weigh``. Every other refusal is clap's own message, from
/// [`message`], with the real name in place of each name that clap
/// suggests and without the names of hidden items, from
/// [`suggestion::real_names`].
pub(super) fn refusal_message(command: &Command, path: &[String], error: &clap::Error) -> String {
    let mut text = command.get_name().to_owned();
    let mut commands = vec![command];
    for name in path {
        text.push(' ');
        text.push_str(name);
        if let Some(subcommand) = commands.last().and_then(|node| node.find_subcommand(name)) {
            commands.push(subcommand);
        }
    }
    if !matches!(
        error.kind(),
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand | ErrorKind::MissingSubcommand
    ) {
        // `message` joins the parts with `; `, and `real_names` changes or
        // drops only a part that starts with clap's suggestion, so splitting
        // on `; ` and joining again keeps every other part as it was.
        return message(&error.render().to_string())
            .split("; ")
            .filter_map(|part| suggestion::real_names(part, &commands))
            .collect::<Vec<_>>()
            .join("; ");
    }
    let node = commands.last().copied().unwrap_or(command);
    let names: Vec<&str> = visible_subcommands(node).map(Command::get_name).collect();
    if names.is_empty() {
        format!("the command `{text}` needs arguments")
    } else {
        format!(
            "the command `{text}` needs a subcommand: {}",
            names.join(", ")
        )
    }
}

/// Returns clap's rendered error text on one line.
///
/// It keeps the text above the `Usage:` line and the `For more information`
/// line, and drops the `error:` and `tip:` labels. The lines of a paragraph
/// join with spaces, and paragraphs and tips join with `; `, as in
/// `unexpected argument '--id' found; a similar argument exists: '--ids'`.
//
// RUST-10: this reads clap's wording and layout, which clap does not
// promise to keep. It misses any part that clap adds below the usage line.
// shelf's golden files fail when the wording changes. A message built from
// the error's kind and its context values, through `Error::context`,
// replaces it once a change makes the text unreadable.
pub(super) fn message(rendered: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut new_part = true;
    for line in rendered.lines() {
        let line = line.trim();
        if line.starts_with("Usage:") || line.starts_with("For more information") {
            break;
        }
        if line.is_empty() {
            new_part = true;
            continue;
        }
        let (line, labelled) = match line
            .strip_prefix("error:")
            .or_else(|| line.strip_prefix("tip:"))
        {
            Some(rest) => (rest.trim_start(), true),
            None => (line, false),
        };
        match parts.last_mut() {
            Some(part) if !new_part && !labelled => {
                part.push(' ');
                part.push_str(line);
            }
            _ => parts.push(line.to_owned()),
        }
        new_part = false;
    }
    parts.join("; ")
}
