//! The word `help`, which crumbkit answers in place of clap: with a topic,
//! or by asking clap for the long help of the command it names.

use std::ffi::OsString;

use clap::Command;

use crate::help::{Guide, topic};

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
/// removes the word and adds `--help` before the first `--`, or at the end
/// when there is none. `shelf help books show` and `shelf books help show`
/// then print what `shelf books show --help` prints, and clap refuses a
/// word that names no command, as in `shelf help comets`.
//
// RUST-10: the request needs the tool's `--help` flag. A tool that disables
// it gets clap's refusal of `--help`. crumbkit's own help flag replaces the
// request if a tool needs that.
pub(super) fn ask_for_help(args: &mut Vec<OsString>, position: usize) {
    args.remove(position);
    let end = args
        .iter()
        .position(|word| word == "--")
        .unwrap_or(args.len());
    args.insert(end, OsString::from("--help"));
}
