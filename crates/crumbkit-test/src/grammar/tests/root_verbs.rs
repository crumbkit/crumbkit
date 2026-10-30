//! Tests for the root verbs that a tool declares on its guide or with
//! `Allow::root_verbs`, which pass the root-leaf rule without an opt-out.

use clap::Command;
use crumbkit::help::Guide;

use crate::Pages;
use crate::allow::{Allow, Rule};
use crate::grammar::check_grammar;
use crate::testing::panic_message;

/// The reason that the tests give for their root verbs.
const REASON: &str = "People and scripts type `start` and `stop` every day.";

/// A guide that declares `start` and `stop` as root verbs.
const GUIDE: Guide = Guide::new().root_verbs(&["start", "stop"], REASON);

/// The opt-outs that declare `start` and `stop` as root verbs at test time.
const ALLOW: &[Allow] = &[Allow::root_verbs(&["start", "stop"], REASON)];

/// A tool with one resource and two commands without subcommands directly
/// under the root.
fn server() -> Command {
    Command::new("server")
        .subcommand(Command::new("logs").subcommand(Command::new("list")))
        .subcommand(Command::new("start"))
        .subcommand(Command::new("stop"))
}

#[test]
fn root_verbs_that_the_guide_declares_pass_the_root_leaf_rule_without_an_opt_out() {
    check_grammar(&server(), Pages::rendered(&GUIDE), &[], &[]);
}

#[test]
fn root_verbs_declared_with_allow_root_verbs_pass_the_root_leaf_rule_without_an_opt_out() {
    check_grammar(&server(), Pages::clap(), &[], ALLOW);
}

#[test]
fn a_root_leaf_that_is_not_declared_still_needs_an_opt_out() {
    const ONE: Guide = Guide::new().root_verbs(&["start"], REASON);

    assert_eq!(
        panic_message(|| check_grammar(&server(), Pages::rendered(&ONE), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         server stop: the command has no subcommands and sits directly under `server`, and a command has the form `server <resource> <verb>`"
    );
}

#[test]
fn a_resource_with_the_name_or_an_alias_of_a_root_verb_fails_without_an_opt_out() {
    const RESOURCE_NAME: &[Allow] = &[Allow::new(Rule::ResourceName, "", "Old names.")];
    let server = server()
        .mut_subcommand("stop", |stop| stop.alias("halt"))
        .subcommand(
            Command::new("jobs")
                .subcommand(Command::new("start").subcommand(Command::new("list")))
                .subcommand(
                    Command::new("pauses")
                        .alias("halt")
                        .subcommand(Command::new("list")),
                ),
        );

    assert_eq!(
        panic_message(|| check_grammar(&server, Pages::rendered(&GUIDE), &[], RESOURCE_NAME))
            .unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         server jobs start: the resource answers to `start`, which the root verb `start` also answers to\n\
         server jobs pauses: the resource answers to `halt`, which the root verb `stop` also answers to\n\
         the opt-out from `grammar/resource-name` for `server` excuses no problem, so it is stale"
    );
}

#[test]
fn a_declared_root_verb_that_names_no_root_leaf_fails_as_a_stale_opt_out_does() {
    const STALE: Guide = Guide::new().root_verbs(&["start", "stop", "logs", "restart"], REASON);

    assert_eq!(
        panic_message(|| check_grammar(&server(), Pages::rendered(&STALE), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the root verb `logs` names no visible command without subcommands directly under `server`, so its declaration is stale\n\
         the root verb `restart` names no visible command without subcommands directly under `server`, so its declaration is stale"
    );
}

#[test]
fn a_declaration_without_root_verbs_fails_as_a_stale_opt_out_does() {
    const EMPTY: &[Allow] = &[Allow::root_verbs(&[], REASON)];
    let server =
        Command::new("server").subcommand(Command::new("logs").subcommand(Command::new("list")));

    assert_eq!(
        panic_message(|| check_grammar(&server, Pages::clap(), &[], EMPTY)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the tool declares root verbs without naming one, so the declaration is stale"
    );
}

#[test]
fn root_verbs_declared_on_the_guide_and_with_allow_root_verbs_fail_the_check() {
    assert_eq!(
        panic_message(|| check_grammar(&server(), Pages::rendered(&GUIDE), &[], ALLOW)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the tool declares root verbs more than once, and a tool declares them once, on its guide or with `Allow::root_verbs`"
    );
}

#[test]
fn an_opt_out_from_the_root_leaf_rule_for_a_declared_root_verb_is_stale() {
    const OPT_OUT: &[Allow] = &[Allow::new(Rule::RootLeaf, "start", "Old habit.")];

    assert_eq!(
        panic_message(|| check_grammar(&server(), Pages::rendered(&GUIDE), &[], OPT_OUT)).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the opt-out from `grammar/root-leaf` for `server start` excuses no problem, so it is stale"
    );
}

#[test]
fn a_root_verb_that_one_declaration_names_twice_fails_the_check() {
    const TWICE: Guide = Guide::new().root_verbs(&["start", "stop", "start"], REASON);

    assert_eq!(
        panic_message(|| check_grammar(&server(), Pages::rendered(&TWICE), &[], &[])).unwrap(),
        "the command tree breaks crumbkit's grammar:\n\
         the root verb `start` is declared more than once, so root help would list it twice"
    );
}
