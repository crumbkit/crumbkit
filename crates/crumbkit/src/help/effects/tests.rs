//! Tests for a leaf's effects: the words help prints and the object that
//! `commands list` writes.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{Effects, Items};
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

/// A row type for the stream leaf of the tests below.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Removal {
    /// The removed planet's name.
    name: String,
}

impl crate::Row for Removal {
    const COLUMNS: &'static [&'static str] = &["name"];
}

/// The guide of `tool()`, whose leaf prints a stream of removals.
const STREAM: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .examples(&[Example::new(&["ceres"], "Remove Ceres")])
    .effects(Effects::destructive().idempotent())
    .stream(crate::Document::<crate::Stream<Removal>>::new(
        "tool",
        "planets-remove",
        1,
        "Each removed planet.",
    ))]);

/// The line that the help of a stream leaf prints after its effects.
const OUTPUT_LINE: &str =
    "Output: one row per line as each arrives, and --json prints one JSON document per line.";

#[test]
fn short_help_of_a_stream_leaf_prints_the_output_line_after_the_effects() {
    let page = help(&STREAM, &["tool", "planets", "remove", "-h"]);

    let expected = format!("Effects: destructive, idempotent, closed-world.\n{OUTPUT_LINE}\n");
    assert!(page.contains(&expected), "{page}");
}

#[test]
fn long_help_of_a_stream_leaf_prints_the_output_line_after_the_effects() {
    let page = help(&STREAM, &["tool", "planets", "remove", "--help"]);

    let expected = format!("Effects: destructive, idempotent, closed-world.\n{OUTPUT_LINE}\n");
    assert!(page.ends_with(&expected), "{page}");
}

/// The guide of `tool()`, whose leaf prints a file's bytes.
const BYTES: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .examples(&[Example::new(&["ceres"], "Remove Ceres")])
    .effects(Effects::destructive().idempotent())
    .document(crate::Document::<crate::Bytes>::new(
        "tool",
        "planets-remove",
        1,
        "The removed planet's file.",
    ))]);

/// The line that the help of a bytes leaf prints after its effects.
const BYTES_LINE: &str = "Output: the bytes unchanged, and --json prints them as text or base64.";

#[test]
fn short_help_of_a_bytes_leaf_prints_the_bytes_output_line_after_the_effects() {
    let page = help(&BYTES, &["tool", "planets", "remove", "-h"]);

    let expected = format!("Effects: destructive, idempotent, closed-world.\n{BYTES_LINE}\n");
    assert!(page.contains(&expected), "{page}");
}

#[test]
fn long_help_of_a_bytes_leaf_prints_the_bytes_output_line_after_the_effects() {
    let page = help(&BYTES, &["tool", "planets", "remove", "--help"]);

    let expected = format!("Effects: destructive, idempotent, closed-world.\n{BYTES_LINE}\n");
    assert!(page.ends_with(&expected), "{page}");
}

#[test]
fn a_leaf_that_prints_one_document_prints_no_output_line() {
    let page = help(&GUIDE, &["tool", "planets", "remove", "--help"]);

    assert!(!page.contains("Output:"), "{page}");
}

/// A row type whose `problem` field marks a problem.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Check {
    /// The planet's name.
    name: String,
    /// What is wrong with the planet, or nothing.
    problem: String,
}

impl crate::Row for Check {
    const COLUMNS: &'static [&'static str] = &["name", "problem"];
    const PROBLEM: Option<&'static str> = Some("problem");
}

/// The guide of `tool()`, whose leaf prints rows that can report a problem.
const PROBLEMS: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .examples(&[Example::new(&["ceres"], "Remove Ceres")])
    .effects(Effects::destructive().idempotent())
    .document(crate::Document::<Vec<Check>>::new(
        "tool",
        "planets-remove",
        1,
        "The removed planets.",
    ))]);

/// The line that the help of a leaf whose rows name a problem field prints
/// after its effects.
const PROBLEMS_LINE: &str = "Problems: a value in the problem column marks a row with a problem, and the command then exits with 1.";

#[test]
fn the_help_of_a_leaf_whose_rows_name_a_problem_field_says_that_a_problem_exits_with_1() {
    let short = help(&PROBLEMS, &["tool", "planets", "remove", "-h"]);
    let long = help(&PROBLEMS, &["tool", "planets", "remove", "--help"]);

    let expected = format!("Effects: destructive, idempotent, closed-world.\n{PROBLEMS_LINE}\n");
    assert!(short.contains(&expected), "{short}");
    assert!(long.ends_with(&expected), "{long}");
}

#[test]
fn the_help_of_a_leaf_whose_rows_name_no_problem_field_has_no_problems_line() {
    let page = help(&STREAM, &["tool", "planets", "remove", "--help"]);

    assert!(!page.contains("Problems:"), "{page}");
}

#[test]
fn replacing_effects_are_destructive_and_report_that_they_replace_data() {
    let effects = Effects::replacing().idempotent();

    assert!(effects.is_destructive());
    assert!(effects.is_replacing());
    assert!(!Effects::destructive().is_replacing());
    assert_eq!(
        effects.line(),
        "Effects: destructive, idempotent, closed-world."
    );
}

#[test]
fn a_write_of_many_items_declares_all_or_none_or_a_result_for_each_item() {
    assert_eq!(Effects::additive().items(), None);
    assert_eq!(
        Effects::destructive().all_or_none().items(),
        Some(Items::AllOrNone)
    );
    assert_eq!(Effects::additive().per_item().items(), Some(Items::PerItem));
}

#[test]
fn the_writes_line_names_each_guard_and_says_when_a_replacing_write_has_no_version_check() {
    let lines = [
        Effects::replacing().writes_line(true),
        Effects::replacing().writes_line(false),
        Effects::additive().writes_line(true),
        Effects::destructive().all_or_none().writes_line(false),
        Effects::replacing().per_item().writes_line(true),
    ];

    assert_eq!(
        lines.map(Option::unwrap_or_default),
        [
            "Writes: replaces existing data; --expect <VERSION> fails with a conflict when the data changed.",
            "Writes: replaces existing data, without a version check.",
            "Writes: --expect <VERSION> fails with a conflict when the data changed.",
            "Writes: applies all items or none.",
            "Writes: replaces existing data; --expect <VERSION> fails with a conflict when the data changed; reports each item and exits 1 when any item fails.",
        ]
    );
}

#[test]
fn a_read_only_command_or_a_write_without_guards_has_no_writes_line() {
    assert_eq!(Effects::read_only().writes_line(false), None);
    assert_eq!(Effects::additive().writes_line(false), None);
    assert_eq!(Effects::destructive().writes_line(false), None);
}

/// The guide of `tool()`, whose leaf replaces data and applies all its
/// items or none.
const REPLACING: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .examples(&[Example::new(&["ceres"], "Remove Ceres")])
    .effects(Effects::replacing().all_or_none())]);

#[test]
fn both_help_pages_print_the_writes_line_after_the_effects_line() {
    let expected = "Effects: destructive, not idempotent, closed-world.\nWrites: replaces existing data, without a version check; applies all items or none.\n";
    for flag in ["-h", "--help"] {
        let page = help(&REPLACING, &["tool", "planets", "remove", flag]);

        assert!(page.contains(expected), "{page}");
    }
}

#[test]
fn the_writes_object_holds_replaces_expect_and_items() {
    let writes = super::writes(Some(Effects::replacing().per_item()), true);
    let json = serde_json::to_string(&writes).unwrap();

    assert_eq!(
        json,
        r#"{"replaces":true,"expect":true,"items":"per_item"}"#
    );
    assert_eq!(super::writes(Some(Effects::read_only()), false), None);
    assert_eq!(super::writes(None, false), None);
}

/// The guide of `tool()`, whose leaf passes on the exit code of the
/// program that removes the planets.
const PASSES_ON: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "remove"])
    .effects(Effects::destructive().idempotent())
    .passes_on_exit("the planet remover")]);

#[test]
fn help_of_a_leaf_that_passes_on_a_program_s_exit_code_names_the_program_after_the_effects() {
    let expected = "Effects: destructive, idempotent, closed-world.\n\
                    Exit code: the command passes on the exit code of the planet remover.\n";
    for flag in ["-h", "--help"] {
        let page = help(&PASSES_ON, &["tool", "planets", "remove", flag]);

        assert!(page.contains(expected), "{page}");
    }
}
