//! Tests for the `-h` call of a refusal whose typed path holds a command
//! that turned its help flag off: the call names the deepest command on the
//! path that keeps a help flag, or the refusal offers no step.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command, value_parser};
use serde_json::{Value, json};

use super::{Parsed, Stop, admits, parse_tree};
use crate::help::{Guide, Leaf, RefusalContext};
use crate::{Call, Sink, parse_to};

/// The reason of the recovery of a refusal whose path offers no help page.
const NO_HELP: &str = "no command that the command line names has a -h help page";

/// Returns the hidden leaf `query`, which turned its help and version flags
/// off and takes every word after `--`, as a leaf that answers a shell's
/// completion requests does.
fn query() -> Command {
    Command::new("query")
        .hide(true)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("words")
                .last(true)
                .num_args(0..)
                .allow_hyphen_values(true)
                .value_parser(value_parser!(OsString)),
        )
}

/// Returns a tool whose group `completion` holds the visible leaf `bash`,
/// the hidden leaf [`query`] and the hidden leaf `debug`, which keeps its
/// help flag.
fn tool() -> Command {
    Command::new("tool").subcommand(
        Command::new("completion")
            .about("Print shell completions")
            .subcommand(Command::new("bash").about("Print the bash script"))
            .subcommand(query())
            .subcommand(
                Command::new("debug").hide(true).arg(
                    Arg::new("verbose")
                        .long("verbose")
                        .action(ArgAction::SetTrue),
                ),
            ),
    )
}

/// Returns a tool that turned its help flag off at the root, with the
/// visible leaf `bash` and the hidden leaf [`query`].
fn tool_without_help() -> Command {
    Command::new("tool")
        .disable_help_flag(true)
        .subcommand(Command::new("bash").about("Print the bash script"))
        .subcommand(query())
}

/// Returns `tool completion bash`, for any refusal.
fn completion_bash(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["completion", "bash"])]
}

/// Returns `tool bash`, for any refusal.
fn bash(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["bash"])]
}

/// Runs `parse_to` over `command` and `guide` with `args` into memory, for
/// a command line that does not parse.
fn parse_guided(command: Command, guide: &Guide, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), command, guide, args);
    let stop = result.err();
    Parsed {
        code: stop.as_ref().map(Stop::code),
        json: stop.as_ref().map(|stop| stop.output().is_json()),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Checks that `words` print a help page on standard output and exit with
/// code 0, so a recovery that names them names a page that exists.
#[track_caller]
fn prints_help(command: Command, words: &[&str]) {
    let parsed = parse_tree(command, words);
    assert_eq!(
        parsed.code,
        Some(ExitCode::SUCCESS),
        "{words:?} print no help page:\n{}",
        parsed.err
    );
    assert!(parsed.out.contains("Usage:"), "{words:?}: {}", parsed.out);
}

#[test]
fn a_refusal_at_a_leaf_without_a_help_flag_names_the_help_of_the_group_above_it() {
    let refused = ["tool", "completion", "query", "--help"];
    admits(
        tool(),
        &refused,
        &["tool", "completion", "query", "--", "--help"],
    );

    let parsed = parse_tree(tool(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed.err.contains("  help:  run `tool completion -h`\n"),
        "{}",
        parsed.err
    );
    assert!(!parsed.err.contains("query -h"), "{}", parsed.err);
    prints_help(tool(), &["tool", "completion", "-h"]);
}

#[test]
fn a_refusal_at_a_hidden_leaf_with_a_help_flag_names_the_leaf_s_own_help() {
    let refused = ["tool", "completion", "debug", "--bogus"];
    admits(tool(), &refused, &refused[..3]);

    let parsed = parse_tree(tool(), &refused);

    assert!(
        parsed
            .err
            .contains("  help:  run `tool completion debug -h`\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_refusal_on_a_path_where_no_command_has_a_help_flag_offers_no_step_and_says_why() {
    let refused = ["tool", "query", "--help"];
    admits(
        tool_without_help(),
        &refused,
        &["tool", "query", "--", "--help"],
    );

    let parsed = parse_tree(tool_without_help(), &refused);

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed
            .err
            .contains(&format!("  help:  no next step: {NO_HELP}\n")),
        "{}",
        parsed.err
    );
    assert!(!parsed.err.contains("-h`"), "{}", parsed.err);
}

#[test]
fn the_refusal_document_of_a_path_without_a_help_flag_has_a_none_recovery_with_its_reason() {
    let refused = ["tool", "query", "--json", "--help"];
    let json = Arg::new("json")
        .long("json")
        .global(true)
        .action(ArgAction::SetTrue);
    admits(
        tool_without_help().arg(json.clone()),
        &refused,
        &["tool", "query", "--json", "--", "--help"],
    );

    let parsed = parse_tree(tool_without_help().arg(json), &refused);

    let document: Value = serde_json::from_str(&parsed.out).unwrap();
    assert_eq!(
        document["recovery"],
        json!({"kind": "none", "reason": NO_HELP})
    );
    assert_eq!(document.get("alternatives"), None);
}

#[test]
fn a_tool_s_call_at_a_leaf_without_a_help_flag_offers_the_group_s_help_as_the_first_alternative() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["completion", "query"]).refusal_alternatives(completion_bash)]);
    let refused = ["tool", "completion", "query", "--help"];
    admits(
        tool(),
        &refused,
        &["tool", "completion", "query", "--", "--help"],
    );

    let parsed = parse_guided(tool(), &GUIDE, &refused);

    assert!(
        parsed
            .err
            .contains("  help:  run `tool completion bash`\n  help:  run `tool completion -h`\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_tool_s_call_on_a_path_without_a_help_flag_is_the_recovery_with_no_help_call_after_it() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["query"]).refusal_alternatives(bash)]);
    let refused = ["tool", "query", "--help"];
    admits(
        tool_without_help(),
        &refused,
        &["tool", "query", "--", "--help"],
    );

    let parsed = parse_guided(tool_without_help(), &GUIDE, &refused);

    assert!(
        parsed.err.contains("  help:  run `tool bash`\n  code:"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_command_with_its_own_short_help_flag_in_place_of_clap_s_is_named_by_the_refusal() {
    let own_help = || {
        tool_without_help().arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .global(true)
                .action(ArgAction::Help),
        )
    };
    let refused = ["tool", "query", "--bogus"];
    admits(own_help(), &refused, &["tool", "query", "--", "--bogus"]);

    let parsed = parse_tree(own_help(), &refused);

    assert!(
        parsed.err.contains("  help:  run `tool query -h`\n"),
        "{}",
        parsed.err
    );
    prints_help(own_help(), &["tool", "query", "-h"]);
}
