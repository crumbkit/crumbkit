//! The dashed form of a command's name, which lets a command named in
//! another style, such as `reading_lists`, also answer to the lowercase
//! dashed words that crumbkit's grammar uses, such as `reading-lists`.

use std::ffi::OsStr;

use clap::Command;

#[cfg(test)]
mod tests;

/// Returns `name` as lowercase ASCII letters and digits in words joined by
/// single dashes, starting with a letter, or `None` when `name` maps to no
/// such form.
///
/// An underscore or a dash between words becomes one dash, and an uppercase
/// letter after a lowercase letter or a digit starts a new word, so
/// `reading_lists`, `readingLists` and `ReadingLists` map to
/// `reading-lists`, and `Books` maps to `books`. A name that is already in
/// that form maps to itself. A name that starts with a digit or a dash, ends
/// with a separator, holds two separators in a row or holds a character
/// other than an ASCII letter, a digit, `_` or `-` maps to nothing, such as
/// `2books`, `-books` and `reading__lists`.
///
/// ```
/// assert_eq!(crumbkit::dashed_name("readingLists").as_deref(), Some("reading-lists"));
/// assert_eq!(crumbkit::dashed_name("2books"), None);
/// ```
//
// RUST-10: a run of capitals stays one word, so `HTTPServer` maps to
// `httpserver`. A rule that splits an acronym from the word after it, as
// the heck crate does, replaces this when a tool names commands that way.
#[must_use]
pub fn dashed_name(name: &str) -> Option<String> {
    let mut dashed = String::with_capacity(name.len() + 4);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c == '_' || c == '-' {
            dashed.push('-');
        } else if c.is_ascii_uppercase() {
            if previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit()) {
                dashed.push('-');
            }
            dashed.push(c.to_ascii_lowercase());
        } else {
            dashed.push(c);
        }
        previous = Some(c);
    }
    is_dashed(&dashed).then_some(dashed)
}

/// Returns the dashed form that the subcommand `name` of `parent` also
/// answers to, or `None` when it answers to none beside its own names.
///
/// A subcommand whose name is not lowercase dashed words, such as
/// `reading_lists`, answers to its [`dashed_name`], such as
/// `reading-lists`, and [`parse`](crate::parse()), completion and
/// [`command_path`](crate::command_path) read that word as the subcommand.
/// Help, completion candidates, the usage log and the parsed words name the
/// subcommand by its own name only. The subcommand answers to no dashed
/// form when its name is already in that form, when its name maps to none,
/// when the form is `help`, which crumbkit reserves for asking for help,
/// when another subcommand of `parent` has the form as its name or one of
/// its aliases, or when another subcommand's name maps to the same form.
///
/// ```
/// use clap::Command;
///
/// let shelf = Command::new("shelf")
///     .subcommand(Command::new("reading_lists"))
///     .subcommand(Command::new("books"));
///
/// assert_eq!(crumbkit::dashed_form(&shelf, "reading_lists").as_deref(), Some("reading-lists"));
/// assert_eq!(crumbkit::dashed_form(&shelf, "books"), None);
/// ```
#[must_use]
pub fn dashed_form(parent: &Command, name: &str) -> Option<String> {
    let form = dashed_name(name)?;
    let reaches = subcommand(parent, OsStr::new(&form)).map(Command::get_name);
    (form != name && reaches == Some(name)).then_some(form)
}

/// Returns the subcommand of `parent` that `word` names: the subcommand
/// whose name or alias is `word`, as `Command::find_subcommand` finds it,
/// or else the subcommand that [`by_form`] finds.
pub(crate) fn subcommand<'a>(parent: &'a Command, word: &OsStr) -> Option<&'a Command> {
    parent
        .find_subcommand(word)
        .or_else(|| by_form(parent, word))
}

/// Returns the one subcommand of `parent` whose name maps to `word` by
/// [`dashed_name`], for a `word` that is no subcommand's name or alias.
/// The word `help`, and a word that the names of two subcommands map to,
/// name no subcommand by a dashed form.
///
/// The caller runs it only when no name or alias matches, so a run that
/// types the tool's own names does no extra work.
pub(crate) fn by_form<'a>(parent: &'a Command, word: &OsStr) -> Option<&'a Command> {
    let word = word.to_str()?;
    if word == "help" || !is_dashed(word) {
        return None;
    }
    let mut matches = parent
        .get_subcommands()
        .filter(|candidate| dashed_name(candidate.get_name()).as_deref() == Some(word));
    let found = matches.next()?;
    matches.next().is_none().then_some(found)
}

/// Reports whether `name` is lowercase ASCII letters and digits in words
/// joined by single dashes, starting with a letter, such as `reading-lists`.
fn is_dashed(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.split('-').all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}
