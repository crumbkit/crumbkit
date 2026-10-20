//! Tests for the walk and the search for `help`: which words they read as
//! an option's values.

use std::ffi::OsString;

use clap::{Arg, ArgAction, Command};

use super::{command_path, help_word, program_stem, walk};
use crate::help::Guide;

/// A tool whose root has a global option, an option with two values, an
/// option with any number of values and a flag, and whose `planets` group,
/// aliased `worlds`, has an option of its own.
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
                .alias("worlds")
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
fn command_path_names_real_commands_and_stops_at_the_first_positional_value() {
    let tool = tool();

    assert_eq!(
        command_path(&tool, &Guide::new(), &["-f", "id", "worlds", "show"]),
        ["planets", "show"]
    );
    assert_eq!(
        command_path(&tool, &Guide::new(), &["planets", "7", "show"]),
        ["planets"]
    );
}

#[test]
fn command_path_follows_a_dashed_form_only_when_the_guide_declares_dashed_forms() {
    let tool = Command::new("tool")
        .subcommand(Command::new("reading_lists").subcommand(Command::new("list")));
    let words = ["reading-lists", "list"];

    assert_eq!(
        command_path(&tool, &Guide::new().dashed_forms(), &words),
        ["reading_lists", "list"]
    );
    assert_eq!(
        command_path(&tool, &Guide::new(), &words),
        Vec::<String>::new()
    );
}

#[test]
fn the_walk_reports_help_typed_as_a_flag_and_not_as_a_value() {
    assert!(walk(&tool(), &args(&["planets", "--help"])).long_help);
    assert!(!walk(&tool(), &args(&["planets", "--", "--help"])).long_help);
}

/// Returns whether the walk over `command` reads `words` as a request for
/// JSON.
fn json(command: &Command, words: &[&str]) -> bool {
    walk(command, &args(words)).json
}

/// Returns the `--json` flag, global when `global` is true.
fn json_flag(global: bool) -> Arg {
    Arg::new("json")
        .long("json")
        .global(global)
        .action(ArgAction::SetTrue)
}

#[test]
fn a_json_word_that_no_command_on_the_path_declares_is_no_request_for_json() {
    assert!(!json(&tool(), &["--json", "planets"]));
    assert!(!json(&tool(), &["planets", "show", "--json"]));
}

#[test]
fn a_global_json_flag_on_the_root_counts_on_every_command_below_it() {
    let tool = tool().arg(json_flag(true));

    assert!(json(&tool, &["planets", "show", "--json"]));
    assert!(json(&tool, &["--json", "planets"]));
}

#[test]
fn a_json_flag_that_only_the_root_declares_counts_only_before_the_subcommand() {
    let tool = tool().arg(json_flag(false));

    assert!(json(&tool, &["--json", "planets"]));
    assert!(!json(&tool, &["planets", "--json"]));
}

#[test]
fn a_json_flag_that_a_subcommand_declares_counts_after_its_name() {
    let tool = tool().mut_subcommand("planets", |planets| planets.arg(json_flag(false)));

    assert!(json(&tool, &["planets", "--json"]));
    assert!(!json(&tool, &["--json", "planets"]));
}

#[test]
fn the_search_for_help_skips_the_value_of_an_option() {
    let found = help_word(
        &tool(),
        &[],
        false,
        &args(&["planets", "--sort", "id", "help"]),
    );

    assert_eq!(found.map(|help| help.position), Some(4));
    assert!(help_word(&tool(), &[], false, &args(&["planets", "--sort", "help"])).is_none());
}

#[test]
fn the_program_path_becomes_its_file_stem_so_help_names_the_tool_without_exe() {
    let mut args = vec![OsString::from("/tools/shelf.exe"), OsString::from("books")];

    program_stem(&mut args);

    assert_eq!(args, ["shelf", "books"]);
}
