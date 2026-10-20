//! The word `help`, which crumbkit answers in place of clap: with a topic,
//! or by asking clap for the long help of the command it names.

use std::ffi::OsString;

use clap::Command;

use crate::help::{Guide, topic};

#[cfg(test)]
mod tests;

/// Reports whether the guide's root `root` in `command` answers the word
/// `help` with topics: it has no command named or aliased `help`, and it has
/// subcommands or the tool declares topics.
pub(super) fn has_topics(command: &Command, root: &[&str], guide: &Guide) -> bool {
    let node = root.iter().fold(command, |node, name| {
        node.find_subcommand(name).unwrap_or(node)
    });
    node.find_subcommand("help").is_none()
        && (node.has_subcommands() || !guide.get_topics().is_empty())
}

/// Returns the topic that `help` at the guide's root prints without clap,
/// for `rest`, the words after `help`: a topic's text when the one word in
/// `rest` that names something names a topic of `guide` or `exit-codes`,
/// and no command or alias directly below the root has that name. It
/// returns `None` otherwise, and the caller asks clap for help instead, so
/// `help` alone prints the root's long help (ADR-0101).
///
/// The words that name something are the words before any `--` that do not
/// start with `-`.
//
// RUST-10: the page ignores flags, so `shelf help catalogue --json` prints
// the topic as text. A topic in JSON replaces this when a caller needs it.
pub(super) fn topic_page(command: &Command, guide: &Guide, rest: &[OsString]) -> Option<String> {
    let mut words = rest
        .iter()
        .take_while(|word| *word != "--")
        .filter(|word| !word.as_encoded_bytes().starts_with(b"-"));
    let root = guide.root_in(command).iter().fold(command, |node, name| {
        node.find_subcommand(name).unwrap_or(node)
    });
    match (words.next(), words.next()) {
        (Some(word), None) if root.find_subcommand(word).is_none() => {
            topic::text(command.get_name(), guide, word.to_str()?)
        }
        _ => None,
    }
}

/// Turns `help` at `position` into a request for clap's long help: it
/// removes the word and adds `--help` directly after the last word that
/// followed `help`, comes before any `--` and does not start with `-`. When
/// no word qualifies, `--help` takes the place of `help`. clap answers
/// `--help` as soon as it meets it, so it prints the help before it reads
/// the flags after the command's words, as it does when the caller types
/// `--help` there. `shelf help books show` and `shelf books help show`
/// print what `shelf books show --help` prints, `shelf help --json` prints
/// what `shelf --help --json` prints, and clap refuses a word that names no
/// command, as in `shelf help comets`. The words after `--` stay as typed.
//
// RUST-10: the search counts the value of an option as a word that names
// something, so `shelf help --from ids.txt` gives clap `--from ids.txt
// --help`, which the root refuses because only `books show` takes
// `--from`. A search that reads each word as `walk` does, with each
// command's options, replaces this when a tool needs it. The request also
// needs the tool's `--help` flag. A tool that disables it gets clap's
// refusal of `--help`. crumbkit's own help flag replaces the request if a
// tool needs that.
pub(super) fn ask_for_help(args: &mut Vec<OsString>, position: usize) {
    args.remove(position);
    let following = args.get(position..).unwrap_or_default();
    let end = following
        .iter()
        .position(|word| word == "--")
        .unwrap_or(following.len());
    let place = following
        .get(..end)
        .unwrap_or_default()
        .iter()
        .rposition(|word| !word.as_encoded_bytes().starts_with(b"-"))
        .map_or(position, |last| position + last + 1);
    args.insert(place, OsString::from("--help"));
}
