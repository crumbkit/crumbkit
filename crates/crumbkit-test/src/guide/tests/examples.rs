//! Tests for the rule that every visible command without subcommands has a
//! leaf with an example, and for its opt-out, `guide/examples`.

use crumbkit::help::{Guide, Leaf};

use super::{LEAVES, check_guide, leaf, panic_message, shelf};
use crate::{Allow, Rule};

#[test]
fn an_opt_out_from_guide_examples_passes_a_leaf_without_examples() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        Leaf::new(&["books", "show"]),
        leaf(&["authors", "list"]),
    ]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideExamples,
        "books show",
        "The command reads a catalogue that no example can name yet.",
    )];

    check_guide(&shelf(), &GUIDE, ALLOW);
}

#[test]
fn an_opt_out_from_guide_examples_covers_the_command_it_names_and_every_command_below_it() {
    const GUIDE: Guide = Guide::new();
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideExamples,
        "books",
        "The books commands get their examples in the next release.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         shelf authors list: no leaf with an example names the command, so its help shows no example"
    );
}

#[test]
fn an_opt_out_from_guide_examples_that_excuses_nothing_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideExamples,
        "books show",
        "The command reads a catalogue that no example can name yet.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the opt-out from `guide/examples` for `shelf books show` excuses no problem, so it is stale"
    );
}

#[test]
fn an_opt_out_from_guide_examples_that_names_no_command_fails_the_check() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideExamples,
        "books export",
        "The command reads a catalogue that no example can name yet.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the opt-out from `guide/examples` for `shelf books export` names no command"
    );
}

#[test]
fn an_opt_out_from_guide_examples_does_not_excuse_another_rule_of_the_check() {
    const GUIDE: Guide = Guide::new().leaves(&[
        leaf(&["books", "list"]),
        leaf(&["books", "show"]),
        leaf(&["authors", "list"]),
        leaf(&["books", "export"]),
    ]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::GuideExamples,
        "books",
        "The books commands get their examples in the next release.",
    )];

    assert_eq!(
        panic_message(|| check_guide(&shelf(), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's guide rules:\n\
         the leaf `books export` names no command\n\
         the opt-out from `guide/examples` for `shelf books` excuses no problem, so it is stale"
    );
}

#[test]
fn the_check_ignores_the_opt_outs_of_other_checks() {
    const GUIDE: Guide = Guide::new().leaves(LEAVES);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::OneName,
        "books",
        "Scripts written before the rename call `shelf titles`.",
    )];

    check_guide(&shelf(), &GUIDE, ALLOW);
}
