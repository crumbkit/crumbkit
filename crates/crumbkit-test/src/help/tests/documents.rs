//! Tests for the rule that a leaf whose effects change something and whose
//! command takes a view flag declares a document or that it prints no
//! result, for its opt-out, `help/document`, and for the rule that a leaf
//! declares only one of the two.

use clap::{Arg, ArgAction, Args as _, Command};
use crumbkit::help::{Effects, Help};
use crumbkit::{Document, Prose, Tool, ViewFlags};

use super::{LEAVES, check_help, leaf, panic_message, shelf};
use crate::{Allow, Rule};

/// The tool of `shelf()` with crumbkit's view flags, which are global
/// arguments of the root, so every command takes them.
fn viewed_shelf() -> Command {
    ViewFlags::augment_args(shelf())
}

/// A tool record whose `shelf books show` declares an additive effect and no
/// document.
const ADDING_SHOW: Tool = Tool::new().help(Help::new().leaves(&[
    leaf(&["books", "list"]),
    leaf(&["books", "show"]).effects(Effects::additive()),
    leaf(&["authors", "list"]),
]));

/// A prose document for `shelf books show`.
const SHOW: Document<Prose<String>> = Document::new("shelf", "books-show", 1, "One book.");

/// The problem that the document rule reports for `shelf books show`.
const SHOW_PROBLEM: &str = "the command tree breaks crumbkit's help rules:\n\
     shelf books show: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream";

/// Returns crumbkit's `--json` view flag as a tool declares it by hand: a
/// global flag with the id `ViewFlags::JSON_ID`.
fn view_json() -> Arg {
    Arg::new(ViewFlags::JSON_ID)
        .long("json")
        .global(true)
        .action(ArgAction::SetTrue)
}

/// Returns a tool's own argument with the id `json` and the long name
/// `long`, which is not a view flag.
fn own_json(long: &'static str) -> Arg {
    Arg::new("json").long(long).action(ArgAction::SetTrue)
}

#[test]
fn a_changing_leaf_without_a_document_fails_when_the_root_takes_the_global_json_view_flag() {
    let tool = shelf().arg(view_json());

    assert_eq!(
        panic_message(|| check_help(&tool, &ADDING_SHOW, &[])).unwrap(),
        SHOW_PROBLEM
    );
}

#[test]
fn a_changing_leaf_without_a_document_passes_when_its_command_takes_no_view_flag() {
    check_help(&shelf(), &ADDING_SHOW, &[]);
}

#[test]
fn a_changing_leaf_without_a_document_passes_when_its_command_has_only_its_own_json_argument() {
    let tool = shelf().mut_subcommand("books", |books| {
        books.mut_subcommand("show", |show| show.arg(own_json("json")))
    });

    check_help(&tool, &ADDING_SHOW, &[]);
}

#[test]
fn a_changing_leaf_without_a_document_fails_when_the_root_takes_the_json_view_flag_and_a_command_above_it_has_its_own_json_argument()
 {
    let tool = shelf()
        .arg(view_json())
        .mut_subcommand("books", |books| books.arg(own_json("from-json")));

    assert_eq!(
        panic_message(|| check_help(&tool, &ADDING_SHOW, &[])).unwrap(),
        SHOW_PROBLEM
    );
}

#[test]
fn an_opt_out_from_tool_document_passes_a_changing_leaf_without_a_document() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::HelpDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    check_help(&viewed_shelf(), &ADDING_SHOW, ALLOW);
}

#[test]
fn an_opt_out_from_tool_document_covers_the_command_it_names_and_every_command_below_it() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]).effects(Effects::additive()),
        leaf(&["books", "show"]).effects(Effects::replacing()),
        leaf(&["authors", "list"]).effects(Effects::additive()),
    ]));
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::HelpDocument,
        "books",
        "The books commands print their own output until they move onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_help(&viewed_shelf(), &TOOL, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf authors list: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream"
    );
}

#[test]
fn an_opt_out_from_tool_document_that_excuses_nothing_fails_the_check() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(LEAVES));
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::HelpDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_help(&viewed_shelf(), &TOOL, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         the opt-out from `help/document` for `shelf books show` excuses no problem, so it is stale"
    );
}

#[test]
fn an_opt_out_from_tool_document_on_a_changing_leaf_whose_command_takes_no_view_flag_excuses_no_problem()
 {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::HelpDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_help(&shelf(), &ADDING_SHOW, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         the opt-out from `help/document` for `shelf books show` excuses no problem, so it is stale"
    );
}

#[test]
fn an_opt_out_from_tool_document_does_not_excuse_the_example_rule() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).effects(Effects::additive()),
    ]));
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::HelpDocument,
        "",
        "The tool prints its own output until it moves onto crumbkit's documents.",
    )];

    assert_eq!(
        panic_message(|| check_help(&viewed_shelf(), &TOOL, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf authors list: no leaf with an example names the command, so its help shows no example"
    );
}

#[test]
fn a_changing_leaf_that_prints_no_result_passes_without_an_opt_out_from_tool_document() {
    const TOOL: Tool = Tool::new().help(
        Help::new().leaves(&[
            leaf(&["books", "list"]),
            leaf(&["books", "show"])
                .effects(Effects::additive())
                .prints_no_result(),
            leaf(&["authors", "list"]),
        ]),
    );

    check_help(&viewed_shelf(), &TOOL, &[]);
}

#[test]
fn a_leaf_that_declares_a_document_and_prints_no_result_fails_the_check() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).document(SHOW).prints_no_result(),
        leaf(&["authors", "list"]),
    ]));

    assert_eq!(
        panic_message(|| check_help(&viewed_shelf(), &TOOL, &[])).unwrap(),
        "the command tree breaks crumbkit's help rules:\n\
         shelf books show: the leaf declares a document and also that its command prints no result, so help and parse follow the document and ignore the declaration; keep only `Leaf::document` or `Leaf::stream` when the command prints a result, or only `Leaf::prints_no_result` when it prints none"
    );
}
