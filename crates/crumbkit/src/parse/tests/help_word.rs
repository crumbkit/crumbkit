//! Tests for the word `help`, which `parse_to` answers in place of clap.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{Parsed, admits, parse, parse_tree, tool};
use crate::Sink;
use crate::help::Guide;
use crate::parse::{Unparsed, parse_to, try_parse_to};

#[test]
fn help_with_a_command_path_prints_that_command_s_long_help() {
    let parsed = parse(&["tool", "help", "planets", "show"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(
        parsed.out,
        parse(&["tool", "planets", "show", "--help"]).out
    );
    assert_eq!(parsed.err, "");
}

#[test]
fn help_without_words_prints_the_root_s_long_help() {
    let parsed = parse(&["tool", "help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, parse(&["tool", "--help"]).out);
    assert_eq!(parsed.err, "");
}

#[test]
fn help_with_a_topic_still_prints_the_topic() {
    let parsed = parse(&["tool", "help", "exit-codes"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert!(parsed.out.contains("  2  "), "{}", parsed.out);
    assert_ne!(parsed.out, parse(&["tool", "--help"]).out);
}

#[test]
fn help_inside_a_group_prints_the_long_help_of_the_command_after_it() {
    let parsed = parse(&["tool", "planets", "help", "show"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(
        parsed.out,
        parse(&["tool", "planets", "show", "--help"]).out
    );
    assert_eq!(parsed.err, "");
}

#[test]
fn help_as_the_last_word_after_a_group_prints_that_group_s_long_help() {
    let parsed = parse(&["tool", "planets", "help"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    assert_eq!(parsed.out, parse(&["tool", "planets", "--help"]).out);
    assert_eq!(parsed.err, "");
}

#[test]
fn help_with_an_unknown_word_is_refused_with_the_root_s_short_help_as_the_recovery() {
    admits(
        tool(),
        &["tool", "help", "plants"],
        &["tool", "help", "planets"],
    );
    let parsed = parse(&["tool", "help", "plants"]);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert_eq!(parsed.out, "");
    let expected = "\
error: unrecognized subcommand 'plants'; a similar subcommand exists: 'planets'
  help:  run `tool -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
}

/// A word as a parsed type, for a tool whose root takes one positional
/// argument.
#[derive(clap::Args)]
struct Word {
    word: String,
}

#[test]
fn a_root_without_subcommands_reads_help_as_an_ordinary_word() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<Word, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Command::new("tool").arg(Arg::new("word").required(true)),
        &Guide::new(),
        ["tool", "help"],
    );

    let Ok((parsed, _)) = result else {
        panic!("the command line did not parse");
    };
    assert_eq!(parsed.word, "help");
}

/// A tool whose `--json` flag sits on the leaf `planets show` alone, so its
/// root and its `planets` group refuse `--json`.
fn leaf_flags() -> Command {
    Command::new("tool").subcommand_required(true).subcommand(
        Command::new("planets")
            .subcommand_required(true)
            .subcommand(
                Command::new("show").arg(Arg::new("json").long("json").action(ArgAction::SetTrue)),
            ),
    )
}

/// Checks that `typed` prints what `asked` prints, with the same exit code
/// and the same answer to whether the output asked for JSON.
#[track_caller]
fn prints_as(typed: &Parsed, asked: &Parsed) {
    assert_eq!(typed.code, asked.code);
    assert_eq!(typed.json, asked.json);
    assert_eq!(typed.out, asked.out);
    assert_eq!(typed.err, asked.err);
}

#[test]
fn a_flag_after_the_bare_help_word_prints_the_root_s_long_help() {
    let parsed = parse_tree(leaf_flags(), &["tool", "help", "--json"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    prints_as(
        &parsed,
        &parse_tree(leaf_flags(), &["tool", "--help", "--json"]),
    );
    assert_eq!(
        parsed.out,
        parse_tree(leaf_flags(), &["tool", "--help"]).out
    );
}

#[test]
fn a_flag_after_help_and_a_command_prints_that_command_s_long_help() {
    let parsed = parse_tree(leaf_flags(), &["tool", "help", "planets", "show", "--json"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    prints_as(
        &parsed,
        &parse_tree(
            leaf_flags(),
            &["tool", "planets", "show", "--help", "--json"],
        ),
    );
}

#[test]
fn a_flag_after_a_command_and_help_prints_that_command_s_long_help() {
    let parsed = parse_tree(leaf_flags(), &["tool", "planets", "help", "--json"]);

    assert_eq!(parsed.code, Some(ExitCode::SUCCESS));
    prints_as(
        &parsed,
        &parse_tree(leaf_flags(), &["tool", "planets", "--help", "--json"]),
    );
    assert_eq!(
        parsed.out,
        parse_tree(leaf_flags(), &["tool", "planets", "--help"]).out
    );
}

/// Returns `words` as the words of a command line.
fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

#[test]
fn help_with_an_unknown_word_and_a_flag_is_refused_with_the_unknown_word_as_the_culprit() {
    let typed = ["tool", "help", "nonsense", "--json"];
    admits(leaf_flags(), &typed, &["tool", "help", "planets", "--json"]);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = try_parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        leaf_flags(),
        &Guide::new(),
        typed,
    );

    let Err(Unparsed::Refused(refused)) = result else {
        panic!("{typed:?} was not refused");
    };
    assert_eq!(refused.typed(), words(&["help", "nonsense", "--json"]));
    assert_eq!(refused.parsed(), words(&["nonsense", "--help", "--json"]));
    assert_eq!(refused.culprit(), Some("nonsense"));
}
