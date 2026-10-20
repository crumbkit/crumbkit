//! Tests for the rule that a leaf whose effects change something and whose
//! command takes a view flag declares a document, and for its opt-out,
//! `guide/document`.

use clap::{Arg, ArgAction, Args as _, Command};
use crumbkit::ViewFlags;
use crumbkit::help::{Effects, Guide};

use super::{LEAVES, check_guide, leaf, panic_message, shelf};
use crate::{Allow, Rule};

/// The tool of `shelf()` with crumbkit's view flags, which are global
/// arguments of the root, so every command takes them.
fn viewed_shelf() -> Command {
    ViewFlags::augment_args(shelf())
}

/// A guide whose `shelf books show` declares an additive effect and no
/// document.
const ADDING_SHOW: Guide = Guide::new().leaves(&[
    leaf(&["books", "list"]),
    leaf(&["books", "show"]).effects(Effects::additive()),
    leaf(&["authors", "list"]),
]);

/// The problem that the document rule reports for `shelf books show`.
const SHOW_PROBLEM: &str = "the command tree breaks crumbkit's guide rules:\n\
     shelf books show: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream";

/// Returns a `--json` flag with the id `json`, global when `global` is set.
fn json(global: bool) -> Arg {
    Arg::new("json")
        .long("json")
        .global(global)
        .action(ArgAction::SetTrue)
}

#[test]
fn a_changing_leaf_without_a_document_fails_when_the_root_takes_global_json() {
    let tool = shelf().arg(json(true));

    assert_eq!(
        panic_message(|| check_guide(&tool, &ADDING_SHOW, &[])).unwrap(),
        SHOW_PROBLEM
    );
}

#[test]
fn a_changing_leaf_without_a_document_passes_when_its_command_takes_no_view_flag() {
    check_guide(&shelf(), &ADDING_SHOW, &[]);
}

#[test]
fn a_changing_leaf_without_a_document_passes_when_its_own_json_is_not_global() {
    let tool = shelf().mut_subcommand("books", |books| {
        books.mut_subcommand("show", |show| show.arg(json(false)))
    });

    check_guide(&tool, &ADDING_SHOW, &[]);
}

#[test]
fn a_changing_leaf_without_a_document_passes_when_a_command_above_it_declares_json_that_is_not_global()
 {
    let tool = shelf()
        .arg(json(true))
        .mut_subcommand("books", |books| books.arg(json(false)));

    check_guide(&tool, &ADDING_SHOW, &[]);
}

#[test]
fn an_opt_out_from_guide_document_passes_a_changing_leaf_without_a_document() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    check_guide(&viewed_shelf(), &ADDING_SHOW, ALLOW);
}

#[test]
fn an_opt_out_from_guide_document_covers_the_command_it_names_and_every_command_below_it() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]).effects(Effects::additive()),
        leaf(&["books", "show"]).effects(Effects::replacing()),
        leaf(&["authors", "list"]).effects(Effects::additive()),
    ]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "books",
        "The books commands print their own output until they move onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&viewed_shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf authors list: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream"
    );
}

#[test]
fn an_opt_out_from_guide_document_that_excuses_nothing_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&viewed_shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the opt-out from `guide/document` for `shelf books show` excuses no problem, so it is stale"
    );
}

#[test]
fn an_opt_out_from_guide_document_on_a_changing_leaf_whose_command_takes_no_view_flag_excuses_no_problem()
 {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &ADDING_SHOW, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the opt-out from `guide/document` for `shelf books show` excuses no problem, so it is stale"
    );
}

#[test]
fn an_opt_out_from_guide_document_does_not_excuse_the_example_rule() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).effects(Effects::additive()),
    ]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "",
        "The tool prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&viewed_shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf authors list: no leaf with an example names the command, so its help shows no example"
    );
}
