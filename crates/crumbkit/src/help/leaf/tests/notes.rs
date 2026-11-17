//! Tests for the notes that a leaf prints after its examples.

use std::process::ExitCode;

use clap::Command;

use super::{EXAMPLES, PLANETS, help, leaves, tree};
use crate::help::{Leaf, extend};
use crate::parse::tests::parse_tree_to;
use crate::{Sink, Tool};

/// The notes of `tool planets list`.
const NOTES: &[&str] = &[
    "Planets that no survey has reached are left out.",
    "The mass is in Earth masses.",
];

#[test]
fn a_leaf_returns_the_notes_it_sets_and_none_when_it_sets_none() {
    let leaf = Leaf::new(&["planets", "list"]).notes(NOTES);

    assert_eq!(leaf.get_notes(), NOTES);
    assert_eq!(Leaf::new(&["planets", "list"]).get_notes(), [] as [&str; 0]);
}

#[test]
fn a_leaf_s_notes_print_after_its_examples_in_short_and_long_help() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .notes(NOTES)]);

    let short = help(&TOOL, &["tool", "planets", "list", "-h"]);
    let long = help(&TOOL, &["tool", "planets", "list", "--help"]);

    assert!(
        short.contains(
            "\
Examples:
  tool planets list
  tool planets list --all

Notes:
  Planets that no survey has reached are left out.
  The mass is in Earth masses.

Global flags:"
        ),
        "{short}"
    );
    assert!(
        long.ends_with(
            "\
  tool planets list --all
          List every planet

Notes:
  Planets that no survey has reached are left out.
  The mass is in Earth masses.
"
        ),
        "{long}"
    );
}

#[test]
fn a_leaf_with_notes_and_no_examples_prints_its_notes_alone() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"]).notes(&["Only one note."])]);

    let short = help(&TOOL, &["tool", "planets", "list", "-h"]);

    assert!(!short.contains("Examples:"), "{short}");
    assert!(
        short.contains("\n\nNotes:\n  Only one note.\n\nGlobal flags:"),
        "{short}"
    );
}

#[test]
fn a_leaf_s_notes_print_before_its_columns_in_long_help() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .notes(&["Only one note."])
        .document(PLANETS)]);

    let long = help(&TOOL, &["tool", "planets", "list", "--help"]);

    let notes = long.find("Notes:").unwrap();
    let columns = long.find("Columns:").unwrap();
    assert!(long.find("Examples:").unwrap() < notes, "{long}");
    assert!(notes < columns, "{long}");
    assert!(long.contains("  Only one note.\n\nColumns:\n"), "{long}");
}

#[test]
fn a_leaf_s_notes_print_once_when_the_tool_called_help_extend_before_parse() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .notes(NOTES)
        .document(PLANETS)]);

    for flag in ["-h", "--help"] {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = parse_tree_to::<()>(
            &mut Sink::new(&mut out, &mut err),
            extend(tree(), &TOOL),
            &TOOL,
            ["tool", "planets", "list", flag],
        );

        assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
        let page = String::from_utf8(out).unwrap();
        assert_eq!(page.matches("Notes:").count(), 1, "{flag}: {page}");
        assert_eq!(
            page,
            help(&TOOL, &["tool", "planets", "list", flag]),
            "{flag}"
        );
    }
}

#[test]
fn extend_gives_a_leaf_its_notes_after_its_examples() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .notes(&["Only one note."])]);

    let mut tree = extend(tree(), &TOOL);
    let list = tree
        .find_subcommand_mut("planets")
        .and_then(|planets| planets.find_subcommand_mut("list"))
        .unwrap();

    let short = list.render_help().to_string();
    assert!(
        short.ends_with("tool planets list --all\n\nNotes:\n  Only one note.\n"),
        "{short}"
    );
}

#[test]
fn a_tool_s_own_after_help_still_prints_before_the_leaf_s_notes() {
    const TOOL: Tool = leaves(&[Leaf::new(&["planets", "list"])
        .examples(EXAMPLES)
        .notes(&["Only one note."])]);
    let mut command: Command = tree();
    command = command.mut_subcommand("planets", |planets| {
        planets.mut_subcommand("list", |list| list.after_help("Own text."))
    });

    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<()>(
        &mut Sink::new(&mut out, &mut err),
        command,
        &TOOL,
        ["tool", "planets", "list", "-h"],
    );

    assert_eq!(result.err().map(ExitCode::from), Some(ExitCode::SUCCESS));
    let page = String::from_utf8(out).unwrap();
    assert!(
        page.contains("Own text.\n\nExamples:\n  tool planets list\n  tool planets list --all\n\nNotes:\n  Only one note.\n"),
        "{page}"
    );
}
