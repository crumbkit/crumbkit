//! Tests for the alternatives that a leaf declares for a refusal of a
//! command line that names it.

use std::process::ExitCode;

use clap::{Args as _, Command};

use super::{Parsed, Stop, admits, tool};
use crate::help::{Guide, Leaf};
use crate::{Call, Page, Sink, Spelling, parse_to};

/// Returns `tool()` with the leaf `planets list`, whose page takes at most
/// 10 rows.
fn bounded() -> Command {
    tool().mut_subcommand("planets", |planets| {
        planets.subcommand(Page::<4, 10>::augment_args(
            Command::new("list").about("List the planets"),
        ))
    })
}

/// The alternative that the leaf `planets list` declares.
const SHOW: &[Call] = &[Call::new(&["planets", "show"])];

/// A guide whose leaf `planets list` offers `tool planets show` when a
/// command line that names it is refused.
const GUIDE: Guide =
    Guide::new().leaves(&[Leaf::new(&["planets", "list"]).refusal_alternatives(SHOW)]);

/// Runs `parse_to` over `bounded()` and [`GUIDE`] with `args` into memory,
/// for a command line that does not parse.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        bounded(),
        &GUIDE,
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
fn a_refusal_of_a_line_that_names_a_leaf_offers_the_leafs_alternatives_after_crumbkits_own() {
    let refused = ["tool", "planets", "list", "--limit", "11"];

    let parsed = parse(&refused);

    let expected = "\
error: invalid value '11' for '--limit <N>': --limit takes at most 10
  help:  run `tool planets list --all`
  help:  run `tool planets list --limit 10`
  help:  run `tool planets show`
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
