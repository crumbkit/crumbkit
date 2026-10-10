//! The rules for one entry of the aliases file, which `expand` and `list`
//! share so they always agree on which entries work.

use clap::Command;
use toml::de::DeValue;

/// Returns whether `word`, typed as the first word after the tool's name,
/// is a real command that an alias of the same name cannot replace: `help`,
/// or a subcommand of the root or one of its aliases, hidden or not.
///
/// clap's `find_subcommand` matches a name or an alias and finds hidden
/// commands too, so the check costs one scan of the root's subcommands.
pub(crate) fn wins(command: &Command, word: &str) -> bool {
    word == "help" || command.find_subcommand(word).is_some()
}

/// Returns the words of an entry's value, or `None` when the value is not a
/// list of one or more strings.
pub(crate) fn words<'a>(value: &'a DeValue<'_>) -> Option<Vec<&'a str>> {
    let list = value.as_array()?;
    if list.is_empty() {
        return None;
    }
    list.iter().map(|word| word.get_ref().as_str()).collect()
}
