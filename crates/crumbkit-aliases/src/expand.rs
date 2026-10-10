//! Replacing a personal alias on the command line with its words.

use std::ffi::OsString;
use std::path::Path;

use clap::Command;
use toml::de::DeTable;

use crate::entry::{wins, words};

#[cfg(test)]
mod tests;

/// Returns the process arguments `args` with a personal alias replaced by
/// its words, for the tool to pass to `crumbkit::parse_from`.
///
/// `command` is the tool's command tree before parsing, and `file` is the
/// path that [`file`](crate::file) returns. Only the first word after the
/// tool's name can be an alias. When the aliases file defines that word as
/// a list of one or more words, those words replace it, and any words typed
/// after it follow them. The words are not expanded again, so an alias whose
/// first word is another alias expands once, and no shell sees them.
///
/// `expand` returns `args` unchanged, without reading the file, when there
/// is no first word, when the first word is not UTF-8 or starts with `-`,
/// and when it is `help` or names a subcommand of the root or one of its
/// aliases, hidden or not. A real command always wins over an alias.
/// Otherwise it reads the file, and it returns `args` unchanged when there
/// is no file, when the file cannot be read or parsed, when the file does
/// not define the word, and when the word's value is not a list of one or
/// more strings.
///
/// `expand` never prints and never fails, because a broken aliases file
/// must not stop the tool's real commands or add text where an agent reads.
/// `<tool> aliases list` reports each problem instead.
///
/// When `expand` replaces an alias, it calls `crumbkit::usage::aliased`, so
/// a tool that keeps crumbkit's usage log records that an alias expanded.
/// The call does nothing in a tool without the log.
pub fn expand<I, T>(command: &Command, args: I, file: Option<&Path>) -> Vec<OsString>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args = args.into_iter().map(Into::into).collect();
    expand_with(command, args, || {
        file.and_then(|path| crate::file::read(path).ok().flatten())
    })
}

/// Returns `args` with the alias in its first word replaced, as [`expand`]
/// describes, calling `read` for the text of the aliases file only when the
/// first word could be an alias.
fn expand_with(
    command: &Command,
    args: Vec<OsString>,
    read: impl FnOnce() -> Option<String>,
) -> Vec<OsString> {
    let Some(word) = args.get(1).and_then(|word| word.to_str()) else {
        return args;
    };
    if word.starts_with('-') || wins(command, word) {
        return args;
    }
    let Some(text) = read() else {
        return args;
    };
    let Ok(table) = DeTable::parse(&text) else {
        return args;
    };
    let Some(replacement) = table
        .get_ref()
        .get(word)
        .and_then(|value| words(value.get_ref()))
    else {
        return args;
    };
    // The usage log records that an alias expanded, and the line holds the
    // expanded command (ADR-0029).
    crumbkit::usage::aliased();
    let mut args = args.into_iter();
    args.next()
        .into_iter()
        .chain(replacement.into_iter().map(OsString::from))
        .chain(args.skip(1))
        .collect()
}
