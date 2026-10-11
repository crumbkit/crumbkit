//! Tests for the walk and the search for `help`: which words they read as
//! an option's values.

use std::ffi::OsString;

use clap::{Arg, ArgAction, Command};

use super::{help_word, walk};

/// A tool whose root has a global option, an option with two values, an
/// option with any number of values and a flag, and whose `planets` group
/// has an option of its own.
fn tool() -> Command {
    Command::new("tool")
        .arg(
            Arg::new("fields")
                .long("fields")
                .short('f')
                .alias("only")
                .global(true),
        )
        .arg(Arg::new("pair").long("pair").num_args(2))
        .arg(Arg::new("many").long("many").num_args(1..))
        .arg(Arg::new("verbose").short('v').action(ArgAction::SetTrue))
        .arg(
            Arg::new("pattern")
                .long("pattern")
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::new("until")
                .long("until")
                .num_args(1..)
                .value_terminator(";"),
        )
        .arg(Arg::new("keyed").long("keyed").require_equals(true))
        .subcommand(
            Command::new("planets")
                .arg(Arg::new("sort").long("sort"))
                .subcommand(Command::new("show")),
        )
}

/// The path of a command line that names no subcommand.
const ROOT: [&str; 0] = [];

/// Returns `words` after the program name as the arguments of `tool`.
fn args(words: &[&str]) -> Vec<OsString> {
    ["tool"].iter().chain(words).map(OsString::from).collect()
}

/// Returns the path that the walk finds in `words`.
fn path(words: &[&str]) -> Vec<String> {
    walk(&tool(), &args(words)).path
}

#[test]
fn the_walk_skips_the_value_of_an_option_that_names_a_subcommand() {
    assert_eq!(path(&["--fields", "planets"]), ROOT);
    assert_eq!(path(&["-f", "planets"]), ROOT);
    assert_eq!(path(&["--only", "planets"]), ROOT);
}

#[test]
fn a_value_in_the_option_s_own_word_leaves_the_next_word_to_the_walk() {
    assert_eq!(path(&["--fields=id", "planets"]), ["planets"]);
    assert_eq!(path(&["-fid", "planets"]), ["planets"]);
    assert_eq!(path(&["-vfid", "planets"]), ["planets"]);
}

#[test]
fn the_walk_reads_an_option_at_the_end_of_a_group_of_short_flags() {
    assert_eq!(path(&["-vf", "planets"]), ROOT);
}

#[test]
fn the_walk_skips_as_many_values_as_an_option_takes() {
    assert_eq!(path(&["--pair", "id", "planets"]), ROOT);
    assert_eq!(path(&["--pair", "id", "name", "planets"]), ["planets"]);
}

#[test]
fn an_option_with_any_number_of_values_takes_words_until_the_next_flag() {
    assert_eq!(path(&["--many", "id", "planets"]), ROOT);
    assert_eq!(path(&["--many", "id", "-v", "planets"]), ["planets"]);
}

#[test]
fn an_option_that_allows_hyphen_values_takes_a_word_that_starts_with_a_hyphen() {
    let walked = walk(&tool(), &args(&["--pattern", "--help", "planets"]));

    assert_eq!(walked.path, ["planets"]);
    assert!(!walked.long_help);
}

#[test]
fn the_value_terminator_ends_an_option_s_values() {
    assert_eq!(path(&["--until", "id", ";", "planets"]), ["planets"]);
}

#[test]
fn an_option_that_requires_equals_takes_no_value_from_the_next_word() {
    assert_eq!(path(&["--keyed", "planets"]), ["planets"]);
}

#[test]
fn the_walk_reads_a_global_option_of_a_command_above_the_current_one() {
    assert_eq!(path(&["planets", "--fields", "show"]), ["planets"]);
    assert_eq!(path(&["planets", "--sort", "show"]), ["planets"]);
}

#[test]
fn the_walk_reports_help_typed_as_a_flag_and_not_as_a_value() {
    assert!(walk(&tool(), &args(&["planets", "--help"])).long_help);
    assert!(!walk(&tool(), &args(&["planets", "--", "--help"])).long_help);
}

#[test]
fn the_search_for_help_skips_the_value_of_an_option() {
    let found = help_word(&tool(), false, &args(&["planets", "--sort", "id", "help"]));

    assert_eq!(found.map(|help| help.position), Some(4));
    assert!(help_word(&tool(), false, &args(&["planets", "--sort", "help"])).is_none());
}
