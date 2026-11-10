//! Checks the words of one command line against a tool's command tree.

use crate::skill::tree::{Argument, Command, Tree};

#[cfg(test)]
mod tests;

/// The flags that clap gives every command, which `commands list` leaves
/// out of its rows.
//
// RUST-10: clap gives `-V` and `--version` only to a command that declares
// a version, and the rows do not say which commands do, so the check
// accepts them on every command. A `version` field on the row replaces this.
const BUILT_IN: [&str; 4] = ["-h", "--help", "-V", "--version"];

/// The word that runs clap's `help` subcommand, which `commands list` leaves
/// out of its rows.
const HELP: &str = "help";

/// Checks `words`, a command line whose first word is the name of `tree`'s
/// root, against `tree`, and returns the reason that it fails.
///
/// The words walk down the tree. A word that names a subcommand moves to it,
/// a flag must be one that the current command or a command above it
/// declares as global, a flag that takes a value must have one, a value must
/// be one of the argument's possible values when it lists them, and every
/// other word fills the next positional argument. A placeholder such as
/// `NAME` or `<name>` counts as any value, and help notation such as
/// `[OPTIONS]`, `[--limit N]` and `ID...` is read as the words it stands
/// for. `--` ends the check, because the words after it are the tool's to
/// pass on.
///
/// A missing required argument passes, because a skill names a command in
/// prose, such as `shelf books show`, without its arguments.
///
/// # Errors
///
/// Returns the reason for the first word that fails, such as ``"`lists` is
/// not a command of `shelf books`"``.
//
// RUST-10: the check reads what a `commands list` row says, so it misses
// the rules that clap applies across arguments: conflicts, groups that need
// one member, and a value count other than one per value name. A command
// alias is no subcommand to it either, because the rows do not list
// aliases. A row that lists them replaces this.
pub(crate) fn verify(tree: &Tree, words: &[String]) -> Result<(), String> {
    let Some(root) = tree.command(tree.name()) else {
        return Ok(());
    };
    let mut walk = Walk {
        tree,
        command: root,
        globals: Vec::new(),
        positional: 0,
    };
    let mut rest = words.iter().skip(1).map(String::as_str);
    while let Some(word) = rest.next() {
        if word == "--" {
            return Ok(());
        }
        let Some(word) = notation(word) else {
            continue;
        };
        if BUILT_IN.contains(&word) {
            continue;
        }
        if let Some(long) = word.strip_prefix("--") {
            let (name, inline) = long.split_once('=').map_or((word, None), |(name, value)| {
                (&word[..name.len() + 2], Some(value))
            });
            let argument = walk.flag(name)?;
            match (argument.value_names.len(), inline) {
                (0, None) => {}
                (0, Some(_)) => return Err(format!("`{name}` takes no value")),
                (_, Some(value)) => value_of(argument, name, value)?,
                (count, None) => {
                    for _ in 0..count {
                        let value = next_value(&mut rest).ok_or_else(|| needs(name))?;
                        value_of(argument, name, value)?;
                    }
                }
            }
        } else if word.len() > 1 && word.starts_with('-') && !is_number(word) {
            walk.shorts(&word[1..], &mut rest)?;
        } else if walk.word(word)? == Next::Stop {
            return Ok(());
        }
    }
    Ok(())
}

/// What the check does after a word that is no flag.
#[derive(Debug, PartialEq, Eq)]
enum Next {
    /// The check goes on to the next word.
    Word,
    /// The rest of the line is not checked.
    Stop,
}

/// The place that a command line has reached in the tree.
struct Walk<'tree> {
    /// The tree.
    tree: &'tree Tree,
    /// The command that the words have reached.
    command: &'tree Command,
    /// The global arguments of the commands above `command`.
    globals: Vec<&'tree Argument>,
    /// The index of the positional argument that the next value fills.
    positional: usize,
}

impl<'tree> Walk<'tree> {
    /// Returns the flag called `name` that the command accepts: its own, or
    /// a global argument of a command above it.
    fn flag(&self, name: &str) -> Result<&'tree Argument, String> {
        self.command
            .arguments
            .iter()
            .chain(self.globals.iter().copied())
            .find(|argument| argument.is_called(name))
            .ok_or_else(|| format!("`{}` has no flag `{name}`", self.command.path))
    }

    /// Checks `letters`, the letters of a word such as `-qn5` after its
    /// dash, as short flags. A flag that takes a value takes the rest of the
    /// word, or the next word when the rest is empty.
    fn shorts<'w>(
        &self,
        letters: &'w str,
        rest: &mut impl Iterator<Item = &'w str>,
    ) -> Result<(), String> {
        for (index, letter) in letters.char_indices() {
            let name = format!("-{letter}");
            let argument = self.flag(&name)?;
            if argument.value_names.is_empty() {
                continue;
            }
            let after = &letters[index + letter.len_utf8()..];
            let value = if after.is_empty() {
                next_value(rest).ok_or_else(|| needs(&name))?
            } else {
                after
            };
            return value_of(argument, &name, value);
        }
        Ok(())
    }

    /// Checks `word`, which is no flag: a subcommand to move to, or a value
    /// for the next positional argument.
    fn word(&mut self, word: &str) -> Result<Next, String> {
        let path = &self.command.path;
        let groups = self.tree.has_subcommands(path);
        let child = format!("{path} {word}");
        if groups && let Some(command) = self.tree.command(&child) {
            self.globals.extend(
                self.command
                    .arguments
                    .iter()
                    .filter(|argument| argument.global),
            );
            self.command = command;
            self.positional = 0;
            return Ok(Next::Word);
        }
        let Some(argument) = self.command.positionals().nth(self.positional) else {
            return if groups && (word == HELP || is_placeholder(word)) {
                // clap's `help` subcommand names any command below it, and a
                // placeholder in place of a subcommand stands for any of
                // them, so the rest of the line is not checked.
                Ok(Next::Stop)
            } else if groups {
                Err(format!("`{word}` is not a command of `{path}`"))
            } else {
                Err(format!("`{path}` takes no argument `{word}`"))
            };
        };
        if !argument.many {
            self.positional += 1;
        }
        value_of(argument, &argument.name, word).map(|()| Next::Word)
    }
}

/// Returns the next word as a flag's value, or `None` when the line ends,
/// the next word is `--`, or the next word is a long flag.
fn next_value<'w>(rest: &mut impl Iterator<Item = &'w str>) -> Option<&'w str> {
    rest.next()
        .and_then(notation)
        .filter(|word| *word != "--" && !word.starts_with("--"))
}

/// Checks that `value` is one of the possible values of `argument`, called
/// `name`, when it lists them. A placeholder passes, and so does each part
/// between commas of a value for an argument that accepts many.
fn value_of(argument: &Argument, name: &str, value: &str) -> Result<(), String> {
    let possible = &argument.possible_values;
    if possible.is_empty() || is_placeholder(value) {
        return Ok(());
    }
    let parts: Vec<&str> = if argument.many {
        value.split(',').collect()
    } else {
        vec![value]
    };
    match parts
        .iter()
        .find(|part| !possible.iter().any(|p| p == *part))
    {
        None => Ok(()),
        Some(part) => {
            let (list, none) = match possible.as_slice() {
                [one] => (one.clone(), "not it"),
                [first, second] => (format!("{first} or {second}"), "neither"),
                [init @ .., last] => (format!("{} or {last}", init.join(", ")), "none of them"),
                [] => (String::new(), "none of them"),
            };
            Err(format!("`{name}` takes {list}, and `{part}` is {none}"))
        }
    }
}

/// Returns the reason that a flag called `name` has no value.
fn needs(name: &str) -> String {
    format!("`{name}` needs a value")
}

/// Returns the word that help notation in `word` stands for, or `None` when
/// the word is notation alone. `...` and `[OPTIONS]` stand for nothing, an
/// optional placeholder such as `[SHELF]` stands for nothing, `ID...` stands
/// for `ID`, and the brackets of `[--limit N]` are dropped.
fn notation(word: &str) -> Option<&str> {
    let word = word.strip_suffix("...").unwrap_or(word);
    if word.is_empty() {
        return None;
    }
    let inner = word.strip_prefix('[').unwrap_or(word);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    let bracketed = inner.len() + 2 == word.len();
    if bracketed && is_placeholder(inner) {
        return None;
    }
    // A bracket still at an end, as in `[--limit` before `N]`, belongs to
    // notation that spans words, so the word without it is checked.
    (!inner.is_empty()).then_some(inner)
}

/// Reports whether `word` is a placeholder that stands for a value: a word
/// in capitals such as `NAME` or `TASK_ID`, or a word in angle brackets
/// such as `<name>`.
fn is_placeholder(word: &str) -> bool {
    let angled = word.len() > 2 && word.starts_with('<') && word.ends_with('>');
    let capitals = word.starts_with(|c: char| c.is_ascii_uppercase())
        && word
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-');
    angled || capitals
}

/// Reports whether `word` is a negative number, such as `-5`, which is a
/// value and not a short flag.
fn is_number(word: &str) -> bool {
    word[1..].parse::<f64>().is_ok()
}
