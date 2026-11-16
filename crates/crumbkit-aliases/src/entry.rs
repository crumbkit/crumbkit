//! The rules for one entry of a source, which `expand`, `names` and `list`
//! share so they always agree on which entries work.

use clap::Command;

use crate::source::Table;

/// The problem of an entry whose key a real command takes.
pub(crate) const SHADOWED: &str =
    "the tool has a command or built-in alias of this name, which wins";

/// The problem of an entry whose key starts with `help` in a tool without a
/// command of that name. crumbkit answers the word `help` itself, before any
/// alias expands.
pub(crate) const HELP: &str = "the tool answers the word help with its own help, which wins";

/// The problem of an entry whose value is not a list of one or more words.
pub(crate) const BAD_VALUE: &str = "the value must be a list of one or more words";

/// The problem of an entry whose key has no word.
pub(crate) const NO_KEY: &str = "the key must be one or more words";

/// The problem of an entry in a lazy source whose key has more than one word
/// and starts with a command. A lazy source is read only when the first word
/// is no command, so such an entry could never match.
pub(crate) const STARTS_WITH_COMMAND: &str =
    "the first word is a command of the tool, and entries of a lazy source cannot start with one";

/// One entry of a source, as every source loads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    /// The words of the key, which a person types after the tool's name.
    pub(crate) key: Vec<String>,
    /// The words that replace the key, or `None` when the entry's value is
    /// not a list of one or more words.
    pub(crate) words: Option<Vec<String>>,
}

impl Item {
    /// Returns the item for `key`, split into words, and `words`.
    pub(crate) fn new(key: &str, words: Option<Vec<String>>) -> Self {
        Self {
            key: key.split_whitespace().map(str::to_owned).collect(),
            words,
        }
    }

    /// Returns the key's words joined by single spaces, as `aliases list`
    /// and the root's help print them.
    pub(crate) fn name(&self) -> String {
        self.key.join(" ")
    }

    /// Returns why this item does not expand against `command`, or `None`
    /// when it does. `deferred` says whether its source is a lazy one, which
    /// `expand` consults only when the first word is no command, so such an
    /// item cannot start with a command and have more words.
    pub(crate) fn problem(&self, command: &Command, deferred: bool) -> Option<&'static str> {
        problem_of(&self.key, self.words.is_some(), command, deferred)
    }
}

/// Returns why an entry with the words `key` does not expand against
/// `command`, or `None` when it does. `has_words` says whether its value is a
/// list of one or more words.
pub(crate) fn problem_of<K: AsRef<str>>(
    key: &[K],
    has_words: bool,
    command: &Command,
    deferred: bool,
) -> Option<&'static str> {
    let Some(first) = key.first().map(AsRef::as_ref) else {
        return Some(NO_KEY);
    };
    if first == "help" {
        return Some(if command.find_subcommand("help").is_none() {
            HELP
        } else {
            SHADOWED
        });
    }
    if deferred && command.find_subcommand(first).is_some() {
        return Some(if key.len() == 1 {
            SHADOWED
        } else {
            STARTS_WITH_COMMAND
        });
    }
    if !deferred && names_a_command(command, key) {
        return Some(SHADOWED);
    }
    (!has_words).then_some(BAD_VALUE)
}

/// Returns whether `command` has a subcommand at the path `key`, each word
/// a name or an alias, hidden or not.
fn names_a_command<K: AsRef<str>>(command: &Command, key: &[K]) -> bool {
    let mut current = command;
    for word in key {
        match current.find_subcommand(word.as_ref()) {
            Some(next) => current = next,
            None => return false,
        }
    }
    true
}

/// Returns whether `word`, typed as the first word after the tool's name,
/// is a real command that a one-word key of a file cannot replace: `help`,
/// or a subcommand of the root or one of its aliases, hidden or not.
///
/// clap's `find_subcommand` matches a name or an alias and finds hidden
/// commands too, so the check costs one scan of the root's subcommands.
pub(crate) fn wins(command: &Command, word: &str) -> bool {
    word == "help" || command.find_subcommand(word).is_some()
}

/// Returns the item of `items` with the longest key that the leading words
/// of `typed` spell and that expands against `command`.
pub(crate) fn longest<'a>(
    items: &'a [Item],
    command: &Command,
    deferred: bool,
    typed: &[&str],
) -> Option<&'a Item> {
    items
        .iter()
        .filter(|item| {
            item.key.len() <= typed.len()
                && item.key.iter().zip(typed).all(|(key, word)| key == word)
                && item.problem(command, deferred).is_none()
        })
        .max_by_key(|item| item.key.len())
}

/// Returns the number of words of the longest key of `table` that the
/// leading words of `typed` spell and that expands against `command`, with
/// its replacement words. It copies nothing: only an entry whose first word
/// is the first typed word builds a list of words.
pub(crate) fn longest_in(
    table: Table,
    command: &Command,
    typed: &[&str],
) -> Option<(usize, &'static [&'static str])> {
    let first = typed.first()?;
    table
        .iter()
        .filter(|(key, _)| key.split_whitespace().next() == Some(first))
        .filter_map(|&(key, words)| {
            let key: Vec<&str> = key.split_whitespace().collect();
            (key.len() <= typed.len()
                && key.iter().zip(typed).all(|(key, word)| key == word)
                && problem_of(&key, true, command, false).is_none())
            .then_some((key.len(), words))
        })
        .max_by_key(|(len, _)| *len)
}
