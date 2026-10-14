use std::ffi::OsString;

use clap::{Arg, ArgAction, Command};

use super::{parsed, refused};
use crate::record::{Name, Run};

/// Returns a tool with a global flag and a global option, a branch with a
/// visible alias, a listing with a long flag, a short-only flag and an
/// option with a default, and a leaf that takes ids.
fn tool() -> Command {
    Command::new("shelf")
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fields")
                .long("fields")
                .global(true)
                .value_name("FIELD"),
        )
        .subcommand(
            Command::new("books")
                .visible_alias("b")
                .subcommand(
                    Command::new("list")
                        .arg(Arg::new("all").long("all").action(ArgAction::SetTrue))
                        .arg(Arg::new("quiet").short('q').action(ArgAction::SetTrue))
                        .arg(Arg::new("limit").long("limit").default_value("4")),
                )
                .subcommand(
                    Command::new("show").arg(
                        Arg::new("ids")
                            .value_name("ID")
                            .num_args(1..)
                            .required(true),
                    ),
                ),
        )
}

/// Returns `line` split into words, after the program name.
fn words(line: &str) -> Vec<OsString> {
    line.split_whitespace().map(OsString::from).collect()
}

/// Returns what the usage log records for `line`, which clap accepts.
fn accepted(line: &str) -> Run {
    let mut command = tool();
    let matches = command.try_get_matches_from_mut(words(line)).unwrap();
    let mut run = Run::default();
    parsed(&mut run, &command, &matches);
    run
}

/// Returns what the usage log records for `line`, which clap refuses, with
/// the command path that the walk in `parse` finds and the flag with the id
/// `nearest` of the command at that path as the nearest flag.
fn rejected(line: &str, path: &[&str], nearest: Option<&str>) -> Run {
    let mut command = tool();
    let _ = command.try_get_matches_from_mut(words(line)).unwrap_err();
    let path: Vec<String> = path.iter().map(|name| (*name).to_owned()).collect();
    let node = path
        .iter()
        .fold(&command, |node, name| node.find_subcommand(name).unwrap());
    let nearest = nearest.map(|id| node.get_arguments().find(|arg| arg.get_id() == id).unwrap());
    let mut run = Run::default();
    refused(&mut run, &command, &path, nearest, &words(line)[1..]);
    run
}

/// Returns the names as text.
fn texts(names: &[Name]) -> Vec<&str> {
    names.iter().map(Name::as_str).collect()
}

#[test]
fn the_command_is_the_path_of_real_names_that_the_caller_reached_through_an_alias() {
    let run = accepted("shelf b list");

    assert_eq!(run.command.unwrap().as_str(), "books list");
}

#[test]
fn a_run_of_the_root_alone_records_no_command() {
    let run = accepted("shelf --json");

    assert_eq!(run.command, None);
    assert_eq!(texts(&run.flags), ["--json"]);
}

#[test]
fn the_flags_are_the_sorted_long_names_typed_at_any_level_without_their_values() {
    let run = accepted("shelf --fields s3cr3t books list --all --json");

    assert_eq!(texts(&run.flags), ["--all", "--fields", "--json"]);
}

#[test]
fn a_flag_without_a_long_name_is_recorded_by_its_short_name() {
    let run = accepted("shelf books list -q");

    assert_eq!(texts(&run.flags), ["-q"]);
}

#[test]
fn a_flag_left_at_its_default_value_is_not_recorded() {
    let run = accepted("shelf books list");

    assert_eq!(run.flags, Vec::<Name>::new());
}

#[test]
fn ids_count_the_values_of_the_command_s_positional_arguments_named_id() {
    assert_eq!(accepted("shelf books show 2 7 9").ids, Some(3));
    assert_eq!(accepted("shelf books list").ids, None);
}

#[test]
fn a_refusal_counts_each_flag_that_names_no_flag_of_the_command_and_names_the_nearest_flag() {
    let run = rejected(
        "shelf books list --al --json --nope=s3cr3t -x",
        &["books", "list"],
        Some("all"),
    );

    assert_eq!(run.command.unwrap().as_str(), "books list");
    assert_eq!(run.unknown_flags, Some(3));
    assert_eq!(run.nearest.unwrap().as_str(), "--all");
}

#[test]
fn a_refusal_without_an_unknown_flag_or_a_suggestion_records_neither() {
    let run = rejected("shelf books show", &["books", "show"], None);

    assert_eq!(run.command.unwrap().as_str(), "books show");
    assert_eq!(run.unknown_flags, None);
    assert_eq!(run.nearest, None);
}

#[test]
fn a_refusal_reads_no_word_after_a_double_dash() {
    let run = rejected("shelf books list -- --nope", &["books", "list"], None);

    assert_eq!(run.unknown_flags, None);
}
