//! Step 4 of the tutorial: pantry's convention tests, which check its
//! command tree and tool record with crumbkit-test.

use clap::CommandFactory;
use crumbkit_test::{Allow, Pages, Rule};

use super::{Cli, TOOL};

/// pantry's opt-outs from crumbkit's conventions, which each check that
/// takes opt-outs reads.
const ALLOW: &[Allow] = &[Allow::new(
    Rule::OneName,
    "jars list",
    "Help has shown `ls` since the first release, and people type it.",
)];

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), Pages::rendered(&TOOL), ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_a_verb() {
    crumbkit_test::check_grammar(&Cli::command(), Pages::rendered(&TOOL), &[], ALLOW);
}

#[test]
fn every_command_declares_its_effects() {
    crumbkit_test::check_effects(&Cli::command(), &TOOL, ALLOW);
}

#[test]
fn every_list_command_pages() {
    crumbkit_test::check_listings(&Cli::command(), ALLOW);
}

#[test]
fn every_command_and_leaf_meets_the_help_rules() {
    crumbkit_test::check_help(&Cli::command(), &TOOL, ALLOW);
}
