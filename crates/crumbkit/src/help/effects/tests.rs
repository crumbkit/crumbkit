//! Tests for a leaf's effects: the words help prints and the object that
//! `commands list` writes.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::Effects;
use crate::help::{Example, Guide, Leaf};
use crate::{Sink, Spelling, parse_to};

/// A tool with a global flag and one leaf, `tool planets remove`.
fn tool() -> Command {
    Command::new("tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("planets").subcommand(
                Command::new("remove")
                    .about("Remove planets by name")
                    .arg(Arg::new("names").value_name("NAME").num_args(1..)),
            ),
        )
}

/// Runs `parse_to` over `tool()` with `guide` for `args`, which ask for
/// help, and returns the page it printed.
fn help(guide: &Guide, args: &[&str]) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        tool(),
        guide,
        Spelling::COLONS,
        args,
    );
    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    String::from_utf8(out).unwrap()
}

/// The guide of `tool()`, whose leaf is destructive and idempotent.
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .examples(&[Example::new(&["ceres"], "Remove Ceres")])
    .effects(Effects::destructive().idempotent())]);

#[test]
fn a_read_only_command_is_idempotent_and_destroys_nothing() {
    let effects = Effects::read_only();

    assert!(effects.is_read_only());
    assert!(!effects.is_destructive());
    assert!(effects.is_idempotent());
    assert!(!effects.is_open_world());
}

#[test]
fn an_additive_command_changes_something_and_destroys_nothing() {
    let effects = Effects::additive();

    assert!(!effects.is_read_only());
    assert!(!effects.is_destructive());
    assert!(!effects.is_idempotent());
}

#[test]
fn a_destructive_command_can_be_idempotent_and_reach_the_open_world() {
    let effects = Effects::destructive().idempotent().open_world();

    assert!(!effects.is_read_only());
    assert!(effects.is_destructive());
    assert!(effects.is_idempotent());
    assert!(effects.is_open_world());
}

#[test]
fn help_names_a_read_only_command_and_its_world() {
    assert_eq!(
        Effects::read_only().line(),
        "Effects: read-only, closed-world."
    );
    assert_eq!(
        Effects::read_only().open_world().line(),
        "Effects: read-only, open-world."
    );
}

#[test]
fn help_names_every_property_of_a_command_that_changes_something() {
    assert_eq!(
        Effects::additive().line(),
        "Effects: additive, not idempotent, closed-world."
    );
    assert_eq!(
        Effects::destructive().idempotent().open_world().line(),
        "Effects: destructive, idempotent, open-world."
    );
}

#[test]
fn the_json_object_holds_the_four_properties_in_mcp_order() {
    let json = serde_json::to_string(&Effects::destructive().idempotent()).unwrap();

    assert_eq!(
        json,
        r#"{"read_only":false,"destructive":true,"idempotent":true,"open_world":false}"#
    );
}

#[test]
fn short_help_prints_the_effects_after_the_examples_and_before_the_global_flags() {
    let expected = "\
Remove planets by name

Usage: tool planets remove [OPTIONS] [NAME]...

Arguments:
  [NAME]...  

Options:
  -h, --help  Print help (see more with '--help')

Examples:
  tool planets remove ceres

Effects: destructive, idempotent, closed-world.
Global flags: --json. `tool planets remove --help` describes them.
";
    assert_eq!(help(&GUIDE, &["tool", "planets", "remove", "-h"]), expected);
}

#[test]
fn long_help_prints_the_effects_after_the_examples() {
    let page = help(&GUIDE, &["tool", "planets", "remove", "--help"]);

    assert!(
        page.ends_with(
            "          Remove Ceres\n\nEffects: destructive, idempotent, closed-world.\n"
        ),
        "{page}"
    );
}

#[test]
fn help_names_each_combination_of_change_repetition_and_world() {
    let lines = [
        Effects::additive().open_world().line(),
        Effects::additive().idempotent().line(),
        Effects::additive().idempotent().open_world().line(),
        Effects::destructive().line(),
        Effects::destructive().open_world().line(),
        Effects::destructive().idempotent().line(),
    ];

    assert_eq!(
        lines,
        [
            "Effects: additive, not idempotent, open-world.",
            "Effects: additive, idempotent, closed-world.",
            "Effects: additive, idempotent, open-world.",
            "Effects: destructive, not idempotent, closed-world.",
            "Effects: destructive, not idempotent, open-world.",
            "Effects: destructive, idempotent, closed-world.",
        ]
    );
}

#[test]
fn a_leaf_without_effects_prints_no_effects_line() {
    const PLAIN: Guide =
        Guide::new()
            .leaves(&[Leaf::new(&["planets", "remove"])
                .examples(&[Example::new(&["ceres"], "Remove Ceres")])]);

    let page = help(&PLAIN, &["tool", "planets", "remove", "-h"]);

    assert!(!page.contains("Effects:"), "{page}");
}
