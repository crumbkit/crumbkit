//! Tests for the word `help` after the program name: `ask_for_help`, which
//! turns it into `--help`, and `topic_page`, which finds the topic that the
//! words after it name.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Arg, Command};

use super::{ask_for_help, topic_page};
use crate::help::{Guide, Topic};
use crate::{Sink, parse_to};

/// A tool with a global option that takes one value, `--units <UNIT>`, and
/// one group.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("units")
                .long("units")
                .short('u')
                .value_name("UNIT")
                .global(true),
        )
        .subcommand(Command::new("planets").subcommand(Command::new("show")))
}

/// A topic of the guide below.
const ORBITS: Topic = Topic::new("orbits", "", "An orbit is one trip around the Sun.\n");

/// A guide with one topic.
const GUIDE: Guide = Guide::new().topics(&[ORBITS]);

/// Runs `parse_to` over `tool()` with `GUIDE`, for `line` that prints a
/// page, and returns the page.
fn page(line: &[&str]) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), &GUIDE, line);

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    assert_eq!(String::from_utf8(err).unwrap(), "");
    String::from_utf8(out).unwrap()
}

/// Returns the words of `line` after `ask_for_help` turned the `help` at
/// `position` into `--help`, reading the words with `command`.
fn asked(command: &Command, line: &[&str], position: usize) -> Vec<OsString> {
    let mut args = words(line);
    ask_for_help(command, &mut args, position);
    args
}

/// Returns `words` as the words of a command line.
fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

#[test]
fn help_goes_after_the_last_command_word_and_before_the_flags() {
    assert_eq!(
        asked(
            &Command::new("tool"),
            &["tool", "help", "planets", "show", "--json"],
            1
        ),
        words(&["tool", "planets", "show", "--help", "--json"])
    );
}

#[test]
fn help_goes_where_the_help_word_was_when_only_flags_follow_it() {
    assert_eq!(
        asked(
            &Command::new("tool"),
            &["tool", "planets", "help", "--json", "--long"],
            2
        ),
        words(&["tool", "planets", "--help", "--json", "--long"])
    );
}

#[test]
fn help_goes_at_the_end_when_the_line_ends_with_a_command_word() {
    assert_eq!(
        asked(
            &Command::new("tool"),
            &["tool", "planets", "help", "show"],
            2
        ),
        words(&["tool", "planets", "show", "--help"])
    );
}

#[test]
fn help_goes_before_a_double_dash_and_leaves_the_words_after_it_untouched() {
    assert_eq!(
        asked(
            &Command::new("tool"),
            &["tool", "help", "planets", "--json", "--", "show", "-x"],
            1
        ),
        words(&["tool", "planets", "--help", "--json", "--", "show", "-x"])
    );
    assert_eq!(
        asked(&Command::new("tool"), &["tool", "help", "--", "planets"], 1),
        words(&["tool", "--help", "--", "planets"])
    );
}

#[test]
fn the_words_before_the_help_word_stay_where_they_are() {
    assert_eq!(
        asked(
            &Command::new("tool"),
            &["tool", "--json", "planets", "help"],
            3
        ),
        words(&["tool", "--json", "planets", "--help"])
    );
}

#[test]
fn help_goes_before_the_separate_value_of_an_option_after_the_command_words() {
    assert_eq!(
        asked(
            &tool(),
            &["tool", "help", "planets", "--units", "metric"],
            1
        ),
        words(&["tool", "planets", "--help", "--units", "metric"])
    );
    assert_eq!(
        asked(
            &tool(),
            &["tool", "planets", "-u", "metric", "help", "show"],
            4
        ),
        words(&["tool", "planets", "-u", "metric", "show", "--help"])
    );
}

#[test]
fn a_topic_followed_by_a_global_option_and_its_separate_value_prints_the_topic() {
    assert_eq!(
        page(&["tool", "help", "orbits", "--units", "metric"]),
        ORBITS.text()
    );
    assert_eq!(
        page(&["tool", "help", "orbits", "-u", "metric"]),
        ORBITS.text()
    );
}

#[test]
fn a_topic_after_a_global_option_and_its_separate_value_prints_the_topic() {
    assert_eq!(
        page(&["tool", "help", "--units", "metric", "orbits"]),
        ORBITS.text()
    );
}

#[test]
fn a_topic_followed_by_an_option_in_the_name_equals_value_form_prints_the_topic() {
    assert_eq!(
        page(&["tool", "help", "orbits", "--units=metric"]),
        ORBITS.text()
    );
}

#[test]
fn a_topic_followed_by_a_flag_prints_the_topic_as_text() {
    let tool = tool().arg(
        Arg::new("json")
            .long("json")
            .global(true)
            .action(clap::ArgAction::SetTrue),
    );
    let mut out = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, Vec::new()),
        tool,
        &GUIDE,
        ["tool", "help", "orbits", "--json"],
    );

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    assert_eq!(String::from_utf8(out).unwrap(), ORBITS.text());
}

#[test]
fn a_value_word_that_names_a_topic_after_an_option_is_the_option_s_value_and_names_no_topic() {
    let args = words(&["tool", "help", "--units", "orbits"]);

    assert_eq!(topic_page(&tool(), &GUIDE, &args, 1), None);
    assert_eq!(
        page(&["tool", "help", "--units", "orbits"]),
        page(&["tool", "--help"])
    );
}

#[test]
fn help_followed_only_by_an_option_and_its_value_prints_the_root_s_help() {
    assert_eq!(
        page(&["tool", "help", "--units", "metric"]),
        page(&["tool", "--help"])
    );
}

#[test]
fn a_word_after_a_double_dash_stays_as_typed_and_names_no_topic() {
    let args = words(&["tool", "help", "--", "orbits"]);

    assert_eq!(topic_page(&tool(), &GUIDE, &args, 1), None);
    assert_eq!(
        asked(&tool(), &["tool", "help", "--", "orbits"], 1),
        words(&["tool", "--help", "--", "orbits"])
    );
}

#[test]
fn a_topic_after_the_words_of_a_guide_root_below_the_clap_root_reads_that_root_s_options() {
    const GUIDE: Guide = Guide::new().root(&["planets"]).topics(&[ORBITS]);
    let tool = Command::new("tool").subcommand(
        Command::new("planets")
            .arg(Arg::new("units").long("units").value_name("UNIT"))
            .subcommand(Command::new("show")),
    );
    let args = words(&["tool", "planets", "help", "orbits", "--units", "metric"]);

    assert_eq!(
        topic_page(&tool, &GUIDE, &args, 2).as_deref(),
        Some(ORBITS.text())
    );
}
