//! The names of a person's aliases that the root's help prints, for a tool
//! that opts in to showing them (ADR-0027).

use clap::Command;
use toml::de::DeTable;

use crate::entry::{wins, words};

#[cfg(test)]
mod tests;

/// Returns the names of the entries of the aliases file of the tool whose
/// clap root is `command` that expand, in name order, for the tool to pass
/// to `crumbkit::help::Guide::personal_aliases`:
///
/// ```
/// use crumbkit::help::Guide;
///
/// const GUIDE: Guide = Guide::new().personal_aliases(crumbkit_aliases::names);
/// ```
///
/// It reads the file that [`file`](crate::file) names for the root's name.
/// An entry that a real command shadows, and an entry whose value is not a
/// list of one or more words, does not expand, so it has no name here, and
/// `<tool> aliases list` reports its problem. A missing file, a file that
/// cannot be read or parsed, and an environment without a home directory
/// give no names. Like [`expand`](crate::expand), it never prints and never
/// fails.
///
/// crumbkit's `parse` calls it only when it prints the root's help, so no
/// other run reads the file.
#[must_use]
pub fn names(command: &Command) -> Vec<String> {
    let text =
        crate::file(command.get_name()).and_then(|path| crate::file::read(&path).ok().flatten());
    working(command, text.as_deref())
}

/// Returns the names of the entries of `text`, the text of the aliases
/// file or `None` when there is none, that expand against `command`, in
/// name order.
fn working(command: &Command, text: Option<&str>) -> Vec<String> {
    let Some(Ok(table)) = text.map(DeTable::parse) else {
        return Vec::new();
    };
    let mut names: Vec<String> = table
        .get_ref()
        .iter()
        .filter(|(name, value)| !wins(command, name.get_ref()) && words(value.get_ref()).is_some())
        .map(|(name, _)| name.get_ref().to_string())
        .collect();
    // A `DeTable` keeps file order when some crate in the build enables
    // toml's `preserve_order` feature, so the names sort here, as the rows
    // of `aliases list` do.
    names.sort();
    names
}
