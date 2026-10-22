//! Tests for the hint that a leaf's function, or the guide's function for a
//! leaf that sets none, returns for a refusal of a command line that names
//! the leaf.

use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Arg, Args as _, Command};
use serde_json::{Value, json};

use super::{Parsed, Stop, Unparsed, admits, tool, try_parse_to};
use crate::help::{Guide, Leaf, RefusalContext};
use crate::{Call, Page, Recovery, Sink, parse_to};

/// The hint of a refused unknown flag at a command that passes its trailing
/// words to a program.
const AFTER_DASHES: &str = "put everything the tool should pass to the program it runs after `--`";

/// Returns `tool()` with the leaf `planets launch`, which takes `--probe`
/// and passes the words after `--` to the launcher, and the leaf `planets
/// list`, whose page takes at most 10 rows.
fn forwarding() -> Command {
    tool().mut_subcommand("planets", |planets| {
        planets
            .subcommand(
                Command::new("launch")
                    .about("Launch a probe")
                    .arg(Arg::new("probe").long("probe").value_name("NAME"))
                    .arg(Arg::new("args").value_name("ARGS").num_args(0..).last(true)),
            )
            .subcommand(Page::<4, 10>::augment_args(
                Command::new("list").about("List the planets"),
            ))
    })
}

/// Returns [`AFTER_DASHES`] for an unknown flag at a command whose last
/// positional takes the words after `--`, and no hint otherwise.
fn after_dashes(context: &RefusalContext<'_>) -> Option<String> {
    let forwards = context.command().get_positionals().any(Arg::is_last_set);
    (forwards && context.kind() == ErrorKind::UnknownArgument).then(|| AFTER_DASHES.to_owned())
}

/// Returns a hint for any refusal.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a refusal hint function returns an `Option`"
)]
fn every_refusal(_: &RefusalContext<'_>) -> Option<String> {
    Some("ask for fewer planets".to_owned())
}

/// Returns `tool planets list` for any refusal.
fn list_planets(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["planets", "list"])]
}

/// A guide whose function [`after_dashes`] serves every command line, and
/// whose leaf `planets launch` also offers `tool planets list`.
const GUIDE: Guide = Guide::new()
    .refusal_hint(after_dashes)
    .leaves(&[Leaf::new(&["planets", "launch"]).refusal_alternatives(list_planets)]);

/// A guide whose function [`every_refusal`] serves every command line,
/// whose leaf `planets launch` sets [`after_dashes`] as its own function,
/// and whose leaf `planets list` offers `tool planets list` too.
const GUIDE_WIDE: Guide = Guide::new().refusal_hint(every_refusal).leaves(&[
    Leaf::new(&["planets", "launch"]).refusal_hint(after_dashes),
    Leaf::new(&["planets", "list"]).refusal_alternatives(list_planets),
]);

/// Runs `parse_to` over `forwarding()` and `guide` with `args` into
/// memory, for a command line that does not parse.
fn parse(guide: &Guide, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        forwarding(),
        guide,
        args,
    );
    let stop = result.err();
    Parsed {
        code: stop.as_ref().map(Stop::code),
        json: stop.as_ref().map(|stop| stop.output().is_json()),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn an_unknown_flag_before_the_dashes_recovers_with_the_hint_and_offers_help_and_the_leafs_calls() {
    let refused = ["tool", "planets", "launch", "--verbose"];

    let parsed = parse(&GUIDE, &refused);

    let expected = "\
error: unexpected argument '--verbose' found; a similar argument exists: '--probe'; to pass '--verbose' as a value, use '-- --verbose'
  usage: tool planets launch --probe <NAME> [-- [ARGS]...]
  help:  put everything the tool should pass to the program it runs after `--`
  help:  run `tool planets launch -h`
  help:  run `tool planets list`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "launch", "--", "--verbose"],
    );
}

#[test]
fn the_refusal_document_names_the_hint_as_the_recovery_and_help_as_the_first_alternative() {
    let refused = ["tool", "planets", "launch", "--json", "--verbose"];

    let parsed = parse(&GUIDE, &refused);

    let document: Value = serde_json::from_str(&parsed.out).unwrap();
    assert_eq!(
        document["recovery"],
        json!({"kind": "hint", "text": AFTER_DASHES})
    );
    assert_eq!(
        document["alternatives"],
        json!([
            {"kind": "run", "call": "tool planets launch -h"},
            {"kind": "run", "call": "tool planets list"},
        ])
    );
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "launch", "--json", "--", "--verbose"],
    );
}

#[test]
fn a_hint_function_that_returns_no_hint_leaves_the_recovery_to_the_leafs_calls() {
    let refused = ["tool", "planets", "launch", "--probe"];

    let parsed = parse(&GUIDE, &refused);

    let expected = "\
error: a value is required for '--probe <NAME>' but none was supplied
  help:  run `tool planets list`
  help:  run `tool planets launch -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "launch", "--probe", "voyager"],
    );
}

#[test]
fn a_leafs_own_hint_function_replaces_the_guides_for_its_command_line() {
    let refused = ["tool", "planets", "launch", "--probe"];

    let parsed = parse(&GUIDE_WIDE, &refused);

    let expected = "\
error: a value is required for '--probe <NAME>' but none was supplied
  help:  run `tool planets launch -h`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "launch", "--probe", "voyager"],
    );
}

#[test]
fn a_limit_above_its_bound_keeps_all_as_the_recovery_and_offers_the_hint_after_crumbkits_own() {
    let refused = ["tool", "planets", "list", "--limit", "11"];

    let parsed = parse(&GUIDE_WIDE, &refused);

    let expected = "\
error: invalid value '11' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list --all`
  help:  run `tool planets list --limit 10`
  help:  ask for fewer planets
  help:  run `tool planets list`
  code:  tool::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "list", "--limit", "10"],
    );
}

#[test]
fn a_refused_line_returned_unprinted_gives_the_hint_as_its_recovery() {
    let refused = ["tool", "planets", "launch", "--verbose"];
    let mut out = Vec::new();
    let mut err = Vec::new();

    let result = try_parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        forwarding(),
        &GUIDE,
        refused,
    );

    let Err(Unparsed::Refused(refusal)) = result else {
        panic!("the command line was not refused");
    };
    assert_eq!(refusal.recovery(), Recovery::Hint(AFTER_DASHES.to_owned()));
    admits(
        forwarding(),
        &refused,
        &["tool", "planets", "launch", "--", "--verbose"],
    );
}
