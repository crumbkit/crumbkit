//! Step 4 of the tutorial: pantry's convention tests, which check its
//! command tree, guide and codes with crumbkit-test.

use clap::CommandFactory;
use crumbkit::Code;
use crumbkit_test::{Allow, Rule};

use super::{Cli, GUIDE, SHELF_NOT_FOUND, SPELLING};

/// pantry's opt-outs from crumbkit's conventions, which each check that
/// takes opt-outs reads.
const ALLOW: &[Allow] = &[Allow::new(
    Rule::OneName,
    "jars list",
    "Help has shown `ls` since the first release, and people type it.",
)];

/// The codes that pantry's failures return.
const CODES: &[&Code] = &[&SHELF_NOT_FOUND];

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_a_verb() {
    crumbkit_test::check_grammar(&Cli::command(), &GUIDE, &[], ALLOW);
}

#[test]
fn every_command_declares_its_effects() {
    crumbkit_test::check_effects(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_list_command_pages() {
    crumbkit_test::check_listings(&Cli::command(), ALLOW);
}

#[test]
fn every_command_and_leaf_meets_the_guide_rules() {
    crumbkit_test::check_guide(&Cli::command(), &GUIDE);
}

#[test]
fn every_failure_code_is_distinct_and_printable() {
    crumbkit_test::check_codes(SPELLING, &[CODES]);
}
