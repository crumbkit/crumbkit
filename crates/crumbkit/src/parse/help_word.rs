//! The word `help`, which crumbkit answers in place of clap: with a topic,
//! or by asking clap for the long help of the command it names.

use std::ffi::OsString;

use clap::Command;

use super::page::print_text;
use super::stop::{Printed, Stop};
use super::walk::{Kind, Words, walk};
use crate::help::{Guide, topic};
use crate::sink::role::Deliver;

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
/// for `args`, the command line with the program name first, whose word
/// `help` is at `position`: a topic's text when the one word after `help`
/// that names something names a topic of `guide` or `exit-codes`, and no
/// command or alias directly below the root has that name. It returns
/// `None` otherwise, and the caller asks clap for help instead, so `help`
/// alone prints the root's long help (ADR-0101).
///
/// [`naming_words`] finds the words that name something, so the separate
/// value of an option, as in `shelf help catalogue --fields title`, names
/// nothing (ADR-0129).
//
// RUST-10: the page ignores flags, so `shelf help catalogue --json` prints
// the topic as text. A topic in JSON replaces this when a caller needs it.
pub(super) fn topic_page(
    command: &Command,
    guide: &Guide,
    args: &[OsString],
    position: usize,
) -> Option<String> {
    let mut words = naming_words(command, args, position)
        .into_iter()
        .filter_map(|index| args.get(index));
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

/// Returns the positions in `args`, the command line with the program name
/// first, of the words after the word `help` at `position` that name
/// something: a command, a topic, or a word that clap refuses.
///
/// It reads every word as the walk does, with the options of the command
/// that the words reach and the global options of the commands above it,
/// and enters each word that names a subcommand. A word that starts with
/// `-` names nothing, and neither does an option's value, also in the
/// separate-word form `--name value`. Reading stops at `--`, so the words
/// after it stay as typed.
fn naming_words(command: &Command, args: &[OsString], position: usize) -> Vec<usize> {
    let mut reader = Words::new(command);
    let mut naming = Vec::new();
    for (index, word) in args.iter().enumerate().skip(1) {
        match reader.read(word) {
            Kind::End => break,
            Kind::Flag | Kind::Value(_) => continue,
            Kind::Name => {}
        }
        if index > position {
            naming.push(index);
        }
        // `help_word` found `help` where no subcommand has that name, so
        // the word `help` itself enters no subcommand.
        if let Some(subcommand) = reader.current.find_subcommand(word) {
            reader.enter(subcommand);
        }
    }
    naming
}

/// Prints `page`, the topic that the word `help` at `position` asked for at
/// the guide's root, to `sink`, and returns the `Stop` of the run, after
/// parsing `args`, the command line with the program name first, against
/// `command` with `guide`.
//
// RUST-10: a topic prints as plain text, also on a terminal and to a
// styled sink, because crumbkit builds it without styles and only the
// pages that clap renders carry them. A topic with the root's heading and
// literal styles, written as `print_page` writes help to a styled sink,
// replaces this when a tool needs colour there.
pub(super) fn print_topic(
    sink: &mut dyn Deliver,
    command: Command,
    guide: &Guide,
    args: &[OsString],
    position: usize,
    page: &str,
) -> Stop {
    let code = print_text(sink, page);
    if crate::events::observed() {
        let root = guide.root_in(&command).iter().map(|&name| name.to_owned());
        crate::events::helped(&command, &root.collect::<Vec<_>>(), true, code);
    }
    // Only this path ends before the walk of the words in `parse`, so it
    // walks the words for `--json` here.
    let json = walk(&command, args).json;
    // A topic prints only at the guide's root, so the path is empty.
    let words = without_help(args, position);
    Stop::new(
        Printed::Topic,
        code,
        json,
        command,
        guide,
        Vec::new(),
        words,
    )
}

/// Turns `help` at `position` into a request for clap's long help: it
/// removes the word and adds `--help` directly after the last word after
/// `help` that [`naming_words`] reads as naming something, reading the
/// words with `command`. When no word qualifies, `--help` takes the place
/// of `help`. clap answers `--help` as soon as it meets it, so it prints
/// the help before it reads the flags and option values after the
/// command's words, as it does when the caller types `--help` there.
/// `shelf help books show` and `shelf books help show` print what `shelf
/// books show --help` prints, `shelf help --json` prints what `shelf
/// --help --json` prints, and clap refuses a word that names no command,
/// as in `shelf help comets`. The words after `--` stay as typed.
//
// RUST-10: the request needs the tool's `--help` flag. A tool that
// disables it gets clap's refusal of `--help`. crumbkit's own help flag
// replaces the request if a tool needs that.
pub(super) fn ask_for_help(command: &Command, args: &mut Vec<OsString>, position: usize) {
    let place = naming_words(command, args, position)
        .last()
        .map_or(position, |&last| last);
    args.insert(place + 1, OsString::from("--help"));
    args.remove(position);
}

/// Returns the words after the program name in `args` without the word
/// `help` at `position`, which crumbkit answers, so the getters of a
/// `Refused` and a `Stop` read the words around it as the caller typed
/// them (ADR-0136).
pub(super) fn without_help(args: &[OsString], position: usize) -> Vec<OsString> {
    args.iter()
        .enumerate()
        .skip(1)
        .filter(|&(index, _)| index != position)
        .map(|(_, word)| word.clone())
        .collect()
}
