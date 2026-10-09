//! Tests for leaves: the examples and columns that `parse_to` adds to the
//! help of a leaf, and the examples that `extend` adds.

use std::borrow::Cow;
use std::cell::Cell;
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;

use super::{Example, Leaf, quote};
use crate::help::{Guide, extend};
use crate::{Document, Row, Sink, Spelling, parse_to};

/// A tool with a global flag and one group of two leaves.
fn tool() -> Command {
    Command::new("tool")
        .about("Explore a small solar system")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print JSON"),
        )
        .subcommand(
            Command::new("planets")
                .about("Work with planets")
                .subcommand_required(true)
                .subcommand(
                    Command::new("list").about("List the planets").arg(
                        Arg::new("all")
                            .long("all")
                            .action(ArgAction::SetTrue)
                            .help("List every planet"),
                    ),
                )
                .subcommand(
                    Command::new("show")
                        .about("Show planets by name")
                        .arg(Arg::new("names").value_name("NAME").num_args(1..)),
                ),
        )
}

/// A row whose every column has a one-paragraph description.
#[derive(Serialize, JsonSchema)]
struct Planet {
    /// The planet's name.
    name: String,
    /// The planet's mass in Earth masses.
    mass: f64,
    /// The number of the planet's known moons.
    moons: u32,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name", "mass"];
    const LONG_COLUMNS: &'static [&'static str] = &["moons"];
}

/// The document of `tool planets list`.
const PLANETS: Document<Vec<Planet>> = Document::new("tool", "planets", 1);

/// The examples of `tool planets list`.
const EXAMPLES: &[Example] = &[
    Example::new(&[], "List the first planets"),
    Example::new(&["--all"], "List every planet"),
];

/// Runs `parse_to` over `tool()` with `guide`, for `args` that ask for help,
/// and returns the help page it printed.
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
    assert_eq!(result.err(), Some(ExitCode::SUCCESS));
    assert_eq!(err, b"");
    String::from_utf8(out).unwrap()
}

#[test]
fn a_word_that_a_posix_shell_would_split_or_expand_is_single_quoted() {
    assert_eq!(quote("--fields=title,year"), "--fields=title,year");
    assert_eq!(quote("./shelf.toml"), "./shelf.toml");
    assert_eq!(quote("Jane Austen"), "'Jane Austen'");
    assert_eq!(quote("$HOME"), "'$HOME'");
    assert_eq!(quote("*.txt"), "'*.txt'");
    assert_eq!(quote("~"), "'~'");
    assert_eq!(quote(""), "''");
    assert_eq!(quote("Ceres's"), r"'Ceres'\''s'");
}

#[test]
fn short_help_lists_the_examples_after_the_options_then_the_global_flags_and_the_next_line() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .document(PLANETS)]);

    let expected = "\
List the planets

Usage: tool planets list [OPTIONS]

Options:
      --all   List every planet
  -h, --help  Print help (see more with '--help')

Examples:
  tool planets list
  tool planets list --all

Global flags: --json. `tool planets list --help` describes them.
Next: tool planets list --help explains each column.
";
    assert_eq!(help(&GUIDE, &["tool", "planets", "list", "-h"]), expected);
}

#[test]
fn long_help_lists_each_example_with_its_description_and_then_the_columns() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .document(PLANETS)]);

    let expected = "\
List the planets

Usage: tool planets list [OPTIONS]

Options:
      --all
          List every planet

      --json
          Print JSON

  -h, --help
          Print help (see a summary with '-h')

Examples:
  tool planets list
          List the first planets

  tool planets list --all
          List every planet

Columns:
  name  The planet's name.
  mass  The planet's mass in Earth masses.

Columns that --long adds:
  moons  The number of the planet's known moons.
";
    assert_eq!(
        help(&GUIDE, &["tool", "planets", "list", "--help"]),
        expected
    );
}

#[test]
fn a_leaf_without_a_document_shows_its_examples_without_a_next_line_or_columns() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"]).examples(EXAMPLES)]);

    let short = help(&GUIDE, &["tool", "planets", "list", "-h"]);
    let long = help(&GUIDE, &["tool", "planets", "list", "--help"]);

    assert!(
        short.ends_with(
            "\
Examples:
  tool planets list
  tool planets list --all

Global flags: --json. `tool planets list --help` describes them.
"
        ),
        "{short}"
    );
    assert!(
        long.ends_with(
            "\
  tool planets list --all
          List every planet
"
        ),
        "{long}"
    );
}

#[test]
fn a_leaf_without_examples_has_no_examples_section() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"]).document(PLANETS)]);

    let short = help(&GUIDE, &["tool", "planets", "list", "-h"]);
    let long = help(&GUIDE, &["tool", "planets", "list", "--help"]);

    assert!(!short.contains("Examples:"), "{short}");
    assert!(
        short.ends_with("Next: tool planets list --help explains each column.\n"),
        "{short}"
    );
    assert!(!long.contains("Examples:"), "{long}");
    assert!(
        long.contains("Print help (see a summary with '-h')\n\nColumns:\n"),
        "{long}"
    );
}

#[test]
fn an_example_without_a_description_prints_its_command_line_alone_in_long_help() {
    const GUIDE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["planets", "show"])
            .examples(&[Example::new(&["Mars", "Ceres's moon"], "")])]);

    let long = help(&GUIDE, &["tool", "planets", "show", "--help"]);

    assert!(
        long.ends_with("Examples:\n  tool planets show Mars 'Ceres'\\''s moon'\n"),
        "{long}"
    );
}

/// A row with a column without a description and a column whose
/// description has two paragraphs.
#[derive(Serialize, JsonSchema)]
struct Moon {
    /// The moon's name.
    ///
    /// A moon without a name prints its provisional designation.
    name: String,
    planet: String,
}

impl Row for Moon {
    const COLUMNS: &'static [&'static str] = &["name", "planet"];
}

/// A guide whose `tool planets show` prints moons.
const MOONS: Guide =
    Guide::new()
        .leaves(&[Leaf::new(&["planets", "show"])
            .document(Document::<Vec<Moon>>::new("tool", "moons", 1))]);

#[test]
fn a_column_without_a_description_prints_its_name_alone() {
    let long = help(&MOONS, &["tool", "planets", "show", "--help"]);

    assert!(long.ends_with("\n  planet\n"), "{long}");
}

#[test]
fn a_column_description_with_several_paragraphs_prints_its_first() {
    let long = help(&MOONS, &["tool", "planets", "show", "--help"]);

    assert!(long.contains("\n  name    The moon's name.\n"), "{long}");
}

thread_local! {
    /// How many times this thread built the JSON Schema of `Counted`.
    static SCHEMAS: Cell<usize> = const { Cell::new(0) };
}

/// A row type that counts how many times its JSON Schema is built.
#[derive(Serialize)]
struct Counted {
    id: u32,
}

impl JsonSchema for Counted {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("Counted")
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        SCHEMAS.set(SCHEMAS.get() + 1);
        json_schema!({ "type": "object", "properties": { "id": { "type": "integer" } } })
    }
}

impl Row for Counted {
    const COLUMNS: &'static [&'static str] = &["id"];
}

#[test]
fn the_schema_is_built_only_when_the_words_ask_for_long_help() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "show"])
        .document(Document::<Vec<Counted>>::new("tool", "counted", 1))]);
    let run = |args: &[&str]| {
        let _ = parse_to::<(), _, _, _, _>(
            &mut Sink::new(Vec::new(), Vec::new()),
            tool(),
            &GUIDE,
            Spelling::COLONS,
            args,
        );
        SCHEMAS.get()
    };

    assert_eq!(run(&["tool", "planets", "show", "Mars"]), 0);
    assert_eq!(run(&["tool", "planets", "show", "-h"]), 0);
    assert_eq!(run(&["tool", "planets", "show", "Mars", "--", "--help"]), 0);
    let after_long_help = run(&["tool", "planets", "show", "--help"]);
    assert!(after_long_help > 0);
    assert!(run(&["tool", "help", "planets", "show"]) > after_long_help);
}

#[test]
fn a_leaf_off_the_typed_path_keeps_its_help() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .document(PLANETS)]);

    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(Vec::new(), Vec::new()),
        tool(),
        &GUIDE,
        Spelling::COLONS,
        ["tool", "planets", "show", "Mars"],
    );

    let Ok(((), command)) = result else {
        panic!("the command line did not parse");
    };
    let list = command
        .find_subcommand("planets")
        .and_then(|planets| planets.find_subcommand("list"))
        .unwrap();
    assert_eq!(list.get_after_help(), None);
    assert_eq!(list.get_after_long_help(), None);
}

#[test]
fn extend_gives_each_leaf_its_examples_and_no_columns() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .document(PLANETS)]);

    let mut tree = extend(tool(), &GUIDE);
    let list = tree
        .find_subcommand_mut("planets")
        .and_then(|planets| planets.find_subcommand_mut("list"))
        .unwrap();

    let short = list.render_help().to_string();
    let long = list.render_long_help().to_string();
    assert!(
        short.ends_with("\n\nExamples:\n  tool planets list\n  tool planets list --all\n"),
        "{short}"
    );
    assert!(
        long.ends_with("\n\n  tool planets list --all\n          List every planet\n"),
        "{long}"
    );
}

#[test]
fn extend_applies_only_the_first_of_two_leaves_with_one_path() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["planets", "show"]).examples(&[Example::new(&["Mars"], "")]),
        Leaf::new(&["planets", "show"]).examples(&[Example::new(&["Venus"], "")]),
    ]);

    let mut tree = extend(tool(), &GUIDE);
    let show = tree
        .find_subcommand_mut("planets")
        .and_then(|planets| planets.find_subcommand_mut("show"))
        .unwrap();

    let short = show.render_help().to_string();
    assert!(short.ends_with("\n  tool planets show Mars\n"), "{short}");
}
