//! The refusal of a command line that clap refused: its one-line message,
//! clap's usage lines and the help page of a group.

use std::ffi::OsString;
use std::fmt::Write as _;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, Command, FromArgMatches};

use super::declared;
use super::lines::visible_subcommands;
use super::suggestion;
use crate::help::Guide;
use crate::refusal::Refusal;

/// Returns the refusal of `words`, a command line that clap refused with
/// `error` at the command at `path` of `command`, with its message, the
/// calls that `guide` declares for it, clap's usage lines, and the page of a
/// group that the line stops at (ADR-0118).
pub(super) fn command_line_refusal<C: FromArgMatches>(
    command: &mut Command,
    guide: &Guide,
    path: &[String],
    error: &clap::Error,
    words: &[OsString],
) -> Refusal {
    let message = refusal_message(command, path, error);
    let page = group_page(command, path, error);
    let usage = usage(&error.render().to_string());
    let declared = declared::alternatives::<C>(command, guide, path, error, words);
    Refusal::command_line(message, command, path, error, words, declared).shown_with(usage, page)
}

/// Returns the message of a refusal of a command line that ends at the
/// command at `path`.
///
/// A command line that stops at a command that needs more, which clap
/// answers with that command's help page or a list of its subcommands, says
/// what the command needs. A group's message names its visible
/// subcommands in the order that the tool declares them, as in
/// ``the command `tool planets` needs a subcommand: show, weigh``.
///
/// Every other refusal is clap's own message, from [`message`], with the
/// real name in place of each name that clap suggests and without the names
/// of hidden items, from [`suggestion::real_names`]. A refusal of missing
/// required arguments separates the missing arguments with commas and lists
/// the visible possible values of each missing argument after its name,
/// from [`list_missing`].
fn refusal_message(command: &Command, path: &[String], error: &clap::Error) -> String {
    let mut text = command.get_name().to_owned();
    let mut commands = vec![command];
    for name in path {
        text.push(' ');
        text.push_str(name);
        if let Some(subcommand) = commands.last().and_then(|node| node.find_subcommand(name)) {
            commands.push(subcommand);
        }
    }
    let node = commands.last().copied().unwrap_or(command);
    if !needs_more(error) {
        let mut rendered = error.render().to_string();
        if error.kind() == ErrorKind::MissingRequiredArgument {
            rendered = list_missing(&rendered, node, error);
        }
        // `message` joins the parts with `; `, and `real_names` changes or
        // drops only a part that starts with clap's suggestion, so splitting
        // on `; ` and joining again keeps every other part as it was.
        return message(&rendered)
            .split("; ")
            .filter_map(|part| suggestion::real_names(part, &commands))
            .collect::<Vec<_>>()
            .join("; ");
    }
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

/// Returns the `-h` page of the group at `path`, the names of the
/// subcommands from the root of `command`, when the command line that clap
/// refused with `error` stopped at that group, as
/// [`refusal_message`] words ``the command `tool planets` needs a
/// subcommand``. It returns `None` for every other refusal, and for a
/// command without visible subcommands, which needs arguments.
///
/// clap's parse built the commands on the path in place and named each
/// after the commands above it, so the page that `Command::render_help`
/// renders is the page that `-h` on the group prints, with crumbkit's help
/// lines. The page is plain text, because the sink's styles describe its
/// output writer and the page prints on its error writer.
fn group_page(command: &mut Command, path: &[String], error: &clap::Error) -> Option<String> {
    if !needs_more(error) {
        return None;
    }
    let mut node = command;
    for name in path {
        if node.find_subcommand(name).is_none() {
            break;
        }
        node = node.find_subcommand_mut(name)?;
    }
    visible_subcommands(node).next()?;
    Some(node.render_help().to_string())
}

/// Reports whether clap refused the command line with `error` because it
/// stopped at a command that needs more: a subcommand or arguments.
fn needs_more(error: &clap::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand | ErrorKind::MissingSubcommand
    )
}

/// Returns clap's usage lines in its rendered error `rendered`, each
/// without clap's `Usage:` label and without the indent that lines up the
/// next forms, as in `tool <NAME>` and `tool <COMMAND>`. They are the lines
/// from the line that starts with `Usage:` to the next blank line, and none
/// when clap printed no usage, as for a refused value or a tool without
/// clap's `usage` feature.
///
/// clap renders its usage after naming the tool by the file name of the
/// program, and `parse` gives clap the file stem on Windows, so the usage
/// names the tool as its help does on every platform.
//
// RUST-10: this reads clap's layout of the usage, which clap does not
// promise to keep. It misses a usage that clap prints without the label.
// shelf's golden files and the tests of `parse` fail when the layout
// changes. Rendering the deepest command's usage with
// `Command::render_usage` replaces it once the layout changes.
pub(super) fn usage(rendered: &str) -> Vec<String> {
    let mut lines = rendered
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("Usage:"));
    let Some(first) = lines.next() else {
        return Vec::new();
    };
    let first = first.trim_start().trim_start_matches("Usage:");
    std::iter::once(first)
        .chain(lines.take_while(|line| !line.trim().is_empty()))
        .map(|line| line.trim().to_owned())
        .collect()
}

/// Returns clap's rendered refusal of missing required arguments with a
/// comma after each missing argument but the last, and with the visible
/// possible values of each missing argument of `node` that has them, in
/// clap's own form, after that argument's name, as in
/// `<SHELL> [possible values: bash, fish, zsh], <DIR>`.
///
/// clap renders each missing argument on its own line as the argument's
/// `Display`, and lists the same text in the error's `InvalidArg` context, so
/// a line that equals one of those names after trimming names that
/// argument. [`message`] joins the lines with spaces, and the commas keep
/// two missing arguments, such as `--edit <APP>` and `<NAME>`, from reading
/// as one option that takes two values. A missing argument that `node` does
/// not hold, such as a required group, gets its comma and no possible
/// values.
//
// RUST-10: this reads clap's layout of the refusal, one argument to a line,
// which clap does not promise to keep. shelf's golden files and the tests of
// `parse` fail when the layout changes. A message built from the error's
// context alone replaces it once the layout changes.
fn list_missing(rendered: &str, node: &Command, error: &clap::Error) -> String {
    let Some(ContextValue::Strings(missing)) = error.get(ContextKind::InvalidArg) else {
        return rendered.to_owned();
    };
    let names = |line: &str| missing.iter().any(|missing| missing == line.trim());
    let lines: Vec<&str> = rendered.split_inclusive('\n').collect();
    let last = lines.iter().rposition(|line| names(line));
    let mut text = String::with_capacity(rendered.len());
    for (index, line) in lines.iter().enumerate() {
        let body = line.trim_end_matches('\n');
        text.push_str(body);
        if names(body) {
            let name = body.trim();
            let values = node
                .get_arguments()
                .find(|arg| arg.to_string() == name)
                .and_then(possible_values);
            if let Some(values) = values {
                text.push(' ');
                text.push_str(&values);
            }
            if Some(index) != last {
                text.push(',');
            }
        }
        text.push_str(&line[body.len()..]);
    }
    text
}

/// Returns the visible possible values of `arg` as clap lists them in the
/// refusal of an invalid value, as in `[possible values: bash, fish, zsh]`,
/// or `None` when it has none. A value that is empty or holds whitespace is
/// quoted, as clap quotes it.
fn possible_values(arg: &Arg) -> Option<String> {
    let mut text = String::new();
    for value in arg.get_possible_values() {
        if value.is_hide_set() {
            continue;
        }
        text.push_str(if text.is_empty() {
            "[possible values: "
        } else {
            ", "
        });
        let name = value.get_name();
        if name.is_empty() || name.contains(char::is_whitespace) {
            // Writing to a `String` cannot fail.
            let _ = write!(text, "{name:?}");
        } else {
            text.push_str(name);
        }
    }
    if text.is_empty() {
        return None;
    }
    text.push(']');
    Some(text)
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
