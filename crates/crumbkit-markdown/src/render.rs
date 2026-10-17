//! Writing the reference: one `CommonMark` file with a section for each
//! command, in the order of the document.

use std::collections::BTreeMap;
// The reference is written to a `String` with `write!`, which cannot fail
// for a `String`, so its result is ignored.
use std::fmt::Write as _;

use crumbkit::help::{self, Effects as HelpEffects};

use crate::document::{self, Argument, Command, Effects, Writes};
use crate::error::{Kind, RenderError};
use crate::text::{anchor, code, fence, one_line, paragraphs, sentence};

#[cfg(test)]
mod tests;

/// The Markdown reference of every command of a tool.
#[derive(Clone, Debug)]
pub struct Reference {
    /// The `CommonMark` text, which ends with one newline.
    text: String,
    /// The number of commands that the reference describes.
    commands: usize,
}

impl Reference {
    /// Returns the `CommonMark` text of the reference, which ends with one
    /// newline.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the number of commands that the reference describes, one
    /// section each.
    #[must_use]
    pub const fn commands(&self) -> usize {
        self.commands
    }
}

/// Returns the Markdown reference of the commands in `document`, the bytes
/// of the `crumbkit/commands-list/v1` document that a tool's
/// `commands list --all --json --full` prints.
///
/// The reference has a title, then one section for each command in the
/// order of the document. A section holds the command's description, links
/// to its subcommands, its arguments, examples and columns, the effects,
/// writes, `Output:` and `Text:` lines that its help prints, and its exit
/// codes. The
/// section's heading is the command line in a code span, and its anchor is
/// the one that GitHub gives that heading.
///
/// # Errors
///
/// Returns an error when `document` is not a `crumbkit/commands-list/v1`
/// document, when it holds one page of a longer listing or no command, or
/// when two headings have one anchor, so that a link would reach the wrong
/// section.
pub fn render(document: &[u8]) -> Result<Reference, RenderError> {
    let commands = document::read(document)?;
    let root = commands.first().map_or("", |command| command.path.as_str());
    check_anchors(root, &commands)?;
    let mut text = format!(
        "# Commands of {}\n\ncrumbkit-markdown writes this reference from the help of {}. \
         Edit the help in the tool's code, and not this file.\n",
        code(root),
        code(root),
    );
    for command in &commands {
        text.push('\n');
        section(&mut text, command, &commands);
    }
    Ok(Reference {
        text,
        commands: commands.len(),
    })
}

/// Returns an error when two headings of the reference have one anchor:
/// the title, or the headings of two commands.
//
// RUST-10: the check covers the title and the command headings, which
// links reach. GitHub numbers a repeated anchor, so a subsection heading
// such as `Exit codes` takes `exit-codes-1` and later; a command whose
// anchor is `exit-codes` keeps its own anchor only when it comes before
// the first such subsection, and the reference does not check for that.
fn check_anchors(root: &str, commands: &[Command]) -> Result<(), RenderError> {
    let mut seen = BTreeMap::new();
    let title = format!("Commands of {root}");
    seen.insert(anchor(&title), title);
    for command in commands {
        let id = anchor(&command.path);
        if let Some(first) = seen.get(&id) {
            return Err(RenderError::new(Kind::AnchorClash {
                anchor: id,
                first: first.clone(),
                second: command.path.clone(),
            }));
        }
        seen.insert(id, command.path.clone());
    }
    Ok(())
}

/// Appends the section of `command` to `text`. `commands` is every command,
/// for the links to its subcommands.
fn section(text: &mut String, command: &Command, commands: &[Command]) {
    let _ = writeln!(text, "## {}", code(&command.path));
    let about = command.long_about.as_ref().or(command.about.as_ref());
    for paragraph in about.map(|about| paragraphs(about)).unwrap_or_default() {
        let _ = write!(text, "\n{paragraph}\n");
    }
    let children = children(command, commands);
    if !children.is_empty() {
        text.push_str("\n### Commands\n\n");
        for child in children {
            let link = format!("[{}](#{})", code(&child.path), anchor(&child.path));
            text.push_str(&item(&link, child.about.as_deref().map(one_line)));
        }
    }
    if !command.arguments.is_empty() {
        text.push_str("\n### Arguments\n\n");
        for argument in &command.arguments {
            text.push_str(&argument_item(argument));
        }
    }
    examples(text, command);
    columns(text, command);
    if let Some(effects) = command.effects {
        let _ = write!(text, "\n{}\n", effects_line(effects));
        if let Some(line) = command
            .writes
            .as_ref()
            .and_then(|writes| writes_line(effects, writes))
        {
            let _ = write!(text, "\n{line}\n");
        }
    }
    match command.form.as_deref() {
        Some("stream") => {
            let _ = write!(text, "\n{}\n", help::STREAM_LINE);
        }
        Some("headed") => {
            let _ = write!(text, "\n{}\n", help::HEADED_LINE);
        }
        Some("bytes") => {
            let _ = write!(text, "\n{}\n", help::BYTES_LINE);
        }
        _ => {}
    }
    if let Some(line) = command
        .line
        .as_deref()
        .filter(|_| command.form.as_deref() != Some("headed"))
    {
        let long = command.columns.iter().any(|column| column.long);
        let _ = write!(text, "\n{}\n", help::text_line(&code(line), long));
    }
    if !command.exits.is_empty() {
        text.push_str("\n### Exit codes\n\n");
        for exit in &command.exits {
            let meaning = sentence(&one_line(&exit.meaning));
            text.push_str(&item(&code(&exit.exit.to_string()), Some(meaning)));
        }
    }
}

/// Returns the commands one word below `command`, in the order of
/// `commands`.
fn children<'a>(command: &Command, commands: &'a [Command]) -> Vec<&'a Command> {
    let prefix = format!("{} ", command.path);
    commands
        .iter()
        .filter(|child| {
            child
                .path
                .strip_prefix(&prefix)
                .is_some_and(|rest| !rest.contains(' '))
        })
        .collect()
}

/// Returns a list item that starts with `head` and, when there is one,
/// continues after a colon with `body`.
fn item(head: &str, body: Option<String>) -> String {
    match body {
        Some(body) if !body.is_empty() => format!("- {head}: {body}\n"),
        Some(_) | None => format!("- {head}\n"),
    }
}

/// Returns the list item of `argument`: its forms, its help, and the facts
/// that help shows about it, such as its default.
fn argument_item(argument: &Argument) -> String {
    let mut name = argument.name.clone();
    if argument.name.starts_with('-') {
        for value in &argument.value_names {
            let _ = write!(name, " <{value}>");
        }
    }
    let mut head = code(&name);
    if let Some(short) = &argument.short {
        head = format!("{}, {head}", code(short));
    }
    let help = argument.long_help.as_ref().or(argument.help.as_ref());
    let mut body = help.map(|help| paragraphs(help)).unwrap_or_default();
    let facts = facts(argument);
    if !facts.is_empty() {
        match body.last_mut() {
            Some(last) => *last = format!("{} {facts}", sentence(last)),
            None => body.push(facts),
        }
    }
    let body: Vec<String> = body
        .iter()
        .map(|paragraph| paragraph.replace('\n', "\n  "))
        .collect();
    item(&head, Some(body.join("\n\n  ")))
}

/// Returns the sentences that describe `argument` beyond its help: whether
/// it is required, whether it takes several values, the values it accepts,
/// its defaults, and whether it works after every subcommand.
fn facts(argument: &Argument) -> String {
    let mut facts = Vec::new();
    match argument.one_of.as_slice() {
        [] if argument.required => facts.push("Required.".to_owned()),
        [] => {}
        [other] => facts.push(format!("Required, or give {} in its place.", code(other))),
        others => facts.push(format!(
            "Required, or give one of {} in its place.",
            and_list(others)
        )),
    }
    if argument.many {
        facts.push("Takes more than one value.".to_owned());
    }
    if !argument.possible_values.is_empty() {
        facts.push(format!("One of {}.", comma_list(&argument.possible_values)));
    }
    match argument.defaults.as_slice() {
        [] => {}
        [default] => facts.push(format!("Default: {}.", code(default))),
        defaults => facts.push(format!("Defaults: {}.", comma_list(defaults))),
    }
    if argument.global {
        facts.push("Works after every subcommand.".to_owned());
    }
    facts.join(" ")
}

/// Returns `values` as code spans separated by commas.
fn comma_list(values: &[String]) -> String {
    let spans: Vec<String> = values.iter().map(|value| code(value)).collect();
    spans.join(", ")
}

/// Returns `values` as code spans separated by commas, with `and` before
/// the last.
fn and_list(values: &[String]) -> String {
    match values.split_last() {
        Some((last, [])) => code(last),
        Some((last, rest)) => format!("{} and {}", comma_list(rest), code(last)),
        None => String::new(),
    }
}

/// Appends the examples of `command` to `text`, each a sentence and a
/// fenced block.
fn examples(text: &mut String, command: &Command) {
    if command.examples.is_empty() {
        return;
    }
    text.push_str("\n### Examples\n");
    for example in &command.examples {
        let about = one_line(&example.about);
        if !about.is_empty() {
            let _ = write!(text, "\n{}\n", sentence(&about));
        }
        let fence = fence(&example.command);
        let _ = write!(text, "\n{fence}sh\n{}\n{fence}\n", example.command);
    }
}

/// Appends the columns of `command` to `text`: the columns that every
/// result has, then the columns that `--long` adds. A headed command's
/// text has no columns, so its names are the JSON fields that `--fields`
/// selects, under a heading that says so, as its help does.
fn columns(text: &mut String, command: &Command) {
    let (long, short): (Vec<_>, Vec<_>) = command.columns.iter().partition(|column| column.long);
    let compact = if command.form.as_deref() == Some("headed") {
        "JSON fields that `--fields` selects"
    } else {
        "Columns"
    };
    for (heading, columns) in [(compact, short), ("Columns that `--long` adds", long)] {
        if columns.is_empty() {
            continue;
        }
        let _ = write!(text, "\n### {heading}\n\n");
        for column in columns {
            let description = column
                .description
                .as_deref()
                .map(|description| sentence(&one_line(description)));
            text.push_str(&item(&code(&column.name), description));
        }
    }
}

/// Returns the `Writes:` line that help prints for a command with
/// `effects` and `writes`, or `None` when help prints none.
fn writes_line(effects: Effects, writes: &Writes) -> Option<String> {
    let mut guards = if writes.replaces {
        HelpEffects::replacing()
    } else {
        help_effects(effects)
    };
    guards = match writes.items.as_deref() {
        Some("all_or_none") => guards.all_or_none(),
        Some("per_item") => guards.per_item(),
        _ => guards,
    };
    guards.writes_line(writes.expect)
}

/// Returns the effects line that help prints for `effects`.
fn effects_line(effects: Effects) -> &'static str {
    help_effects(effects).line()
}

/// Returns `effects` as crumbkit's help builds them.
fn help_effects(effects: Effects) -> HelpEffects {
    let mut line = if effects.read_only {
        HelpEffects::read_only()
    } else if effects.destructive {
        HelpEffects::destructive()
    } else {
        HelpEffects::additive()
    };
    if effects.idempotent {
        line = line.idempotent();
    }
    if effects.open_world {
        line = line.open_world();
    }
    line
}
