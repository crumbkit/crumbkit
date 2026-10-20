//! Tests for the rule that a leaf whose effects change something declares a
//! document, and for its opt-out, `guide/document`.

use crumbkit::help::{Effects, Guide};

use super::{LEAVES, check_guide, leaf, panic_message, shelf};
use crate::{Allow, Rule};

#[test]
fn an_opt_out_from_guide_document_passes_a_changing_leaf_without_a_document() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]).effects(Effects::additive()),
        leaf(&["authors", "list"]),
    ]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideDocument,
        "books show",
        "The command prints its own output until it moves onto crumbkit's documents.",
    )];

    check_guide(&shelf(), &GUIDE, ALLOW);
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
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
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
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
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
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf authors list: no leaf with an example names the command, so its help shows no example"
    );
}
