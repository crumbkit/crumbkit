//! Tests for the alternatives that a leaf's function, or the guide's
//! function for a leaf that sets none, returns for a refusal of a command
//! line that names the leaf.

use std::process::ExitCode;

use clap::{Arg, Args as _, Command};
use serde_json::{Value, json};

use super::{Parsed, Stop, admits, tool};
use crate::help::{Guide, Leaf, RefusalContext};
use crate::{Call, Page, Sink, Spelling, parse_to};

/// The planets that `tool planets orbit` knows.
const KNOWN: &[&str] = &["mars", "venus"];

/// Returns `tool()` with the leaf `planets list`, whose page takes at most
/// 10 rows, and the leaf `planets orbit`, whose one positional takes the
/// name of a planet in [`KNOWN`].
fn bounded() -> Command {
    tool().mut_subcommand("planets", |planets| {
        planets
            .subcommand(Page::<4, 10>::augment_args(
                Command::new("list").about("List the planets"),
            ))
            .subcommand(
                Command::new("orbit").about("Show a planet's orbit").arg(
                    Arg::new("name")
                        .value_name("NAME")
                        .required(true)
                        .value_parser(|name: &str| {
                            if KNOWN.contains(&name) {
                                Ok(name.to_owned())
                            } else {
                                Err(format!("no planet is named {name}"))
                            }
                        }),
                ),
            )
    })
}

/// Returns `tool planets show Mars` for any refusal of `tool planets list`.
fn show_mars(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["planets", "show"]).value("names", "Mars")]
}

/// Returns a call that names no command, then the refused line of `tool
/// planets orbit` with the extension cut from the refused value, such as
/// `mars` for `mars.json`.
fn without_extension(context: &RefusalContext<'_>) -> Vec<Call> {
    let mut calls = vec![Call::new(&["planets", "orbt"])];
    if context.argument() == Some("name")
        && let Some(value) = context.value()
        && let Some((stem, _)) = value.split_once('.')
    {
        calls.push(Call::new(&["planets", "orbit"]).value("name", stem));
    }
    calls
}

/// A guide whose leaf `planets list` offers `tool planets show Mars`, and
/// whose leaf `planets orbit` offers its refused line with the refused
/// value corrected.
const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["planets", "list"]).refusal_alternatives(show_mars),
    Leaf::new(&["planets", "orbit"]).refusal_alternatives(without_extension),
]);

/// Returns the calls of [`without_extension`] and then `tool planets list`,
/// for a refusal of any command line.
fn every_command(context: &RefusalContext<'_>) -> Vec<Call> {
    let mut calls = without_extension(context);
    calls.push(Call::new(&["planets", "list"]));
    calls
}

/// A guide whose function [`every_command`] serves every command line, and
/// whose leaf `planets list` sets its own function.
const GUIDE_WIDE: Guide = Guide::new()
    .refusal_alternatives(every_command)
    .leaves(&[Leaf::new(&["planets", "list"]).refusal_alternatives(show_mars)]);

/// Returns no call, for any refusal.
fn nothing(_: &RefusalContext<'_>) -> Vec<Call> {
    Vec::new()
}

/// A guide whose leaf `planets show` sets a function that returns no call.
const GUIDE_SILENT: Guide =
    Guide::new().leaves(&[Leaf::new(&["planets", "show"]).refusal_alternatives(nothing)]);

/// Runs `parse_to` over `bounded()` and [`GUIDE`] with `args` into memory,
/// for a command line that does not parse.
fn parse(args: &[&str]) -> Parsed {
    parse_guided(&GUIDE, args)
}

/// Runs `parse_to` over `bounded()` and `guide` with `args` into memory,
/// for a command line that does not parse.
fn parse_guided(guide: &Guide, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        bounded(),
        guide,
        Spelling::COLONS,
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
fn a_limit_above_its_bound_keeps_all_as_the_recovery_and_offers_the_leafs_calls_after_crumbkits_own()
 {
    let refused = ["tool", "planets", "list", "--limit", "11"];

    let parsed = parse(&refused);

    let expected = "\
error: invalid value '11' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list --all`
  help:  run `tool planets list --limit 10`
  help:  run `tool planets show Mars`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "list", "--limit", "10"],
    );
}

#[test]
fn a_refusal_recovers_with_the_corrected_line_of_the_leafs_function_and_offers_help_first() {
    let refused = ["tool", "planets", "orbit", "mars.json"];

    let parsed = parse(&refused);

    let expected = "\
error: invalid value 'mars.json' for '<NAME>': no planet is named mars.json
  help:  run `tool planets orbit mars`
  help:  run `tool planets orbit -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(bounded(), &refused, &["tool", "planets", "orbit", "mars"]);
}

#[test]
fn the_refusal_document_names_the_corrected_line_as_the_recovery_and_help_as_the_first_alternative()
{
    let refused = ["tool", "planets", "orbit", "mars.json", "--json"];

    let parsed = parse(&refused);

    let document: Value = serde_json::from_str(&parsed.out).unwrap();
    assert_eq!(
        document["recovery"],
        json!({"kind": "run", "call": "tool planets orbit mars"})
    );
    assert_eq!(
        document["alternatives"],
        json!([{"kind": "run", "call": "tool planets orbit -h"}])
    );
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "orbit", "mars", "--json"],
    );
}

#[test]
fn a_function_that_returns_no_call_leaves_help_as_the_recovery() {
    let refused = ["tool", "planets", "show", "Mars", "--srot", "name"];

    let parsed = parse_guided(&GUIDE_SILENT, &refused);

    let expected = "\
error: unexpected argument '--srot' found; a similar argument exists: '--sort'
  help:  run `tool planets show -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "show", "Mars", "--sort", "name"],
    );
}

#[test]
fn a_function_whose_calls_do_not_render_or_parse_leaves_help_as_the_recovery_without_a_warning() {
    let refused = ["tool", "planets", "orbit", "pluto.json"];

    let parsed = parse(&refused);

    let expected = "\
error: invalid value 'pluto.json' for '<NAME>': no planet is named pluto.json
  help:  run `tool planets orbit -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(bounded(), &refused, &["tool", "planets", "orbit", "venus"]);
}

#[test]
fn a_refusal_of_a_line_that_names_another_command_offers_none_of_the_leafs_alternatives() {
    let refused = ["tool", "planets", "show", "Mars", "--srot", "name"];

    let parsed = parse(&refused);

    let expected = "\
error: unexpected argument '--srot' found; a similar argument exists: '--sort'
  help:  run `tool planets show -h`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "show", "Mars", "--sort", "name"],
    );
}

#[test]
fn a_refusal_of_a_line_whose_command_sets_no_function_recovers_with_the_guides_first_call() {
    let refused = ["tool", "planets", "orbit", "mars.json"];

    let parsed = parse_guided(&GUIDE_WIDE, &refused);

    let expected = "\
error: invalid value 'mars.json' for '<NAME>': no planet is named mars.json
  help:  run `tool planets orbit mars`
  help:  run `tool planets orbit -h`
  help:  run `tool planets list`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(bounded(), &refused, &["tool", "planets", "orbit", "mars"]);
}

#[test]
fn a_leafs_own_function_replaces_the_guides_function_for_its_command_line() {
    let refused = ["tool", "planets", "list", "--limit", "11"];

    let parsed = parse_guided(&GUIDE_WIDE, &refused);

    let expected = "\
error: invalid value '11' for '--limit <N>': --limit takes a whole number from 1 to 10
  help:  run `tool planets list --all`
  help:  run `tool planets list --limit 10`
  help:  run `tool planets show Mars`
  code:  crumbkit::cli::usage
";
    assert_eq!(parsed.err, expected);
    admits(
        bounded(),
        &refused,
        &["tool", "planets", "list", "--limit", "10"],
    );
}
