//! Tests for cargo-crumbkit as a whole, which hold it to the conventions it
//! measures in other tools.

use clap::CommandFactory as _;
use crumbkit_test::{Allow, Budgets};

use crate::SPELLING;
use crate::cli::{Cli, GUIDE};
use crate::error::WeighError;

/// cargo-crumbkit's opt-outs from crumbkit's conventions, which every check
/// below reads. It has none.
const ALLOW: &[Allow] = &[];

/// The budgets of cargo-crumbkit's `-h` pages, which are crumbkit's
/// defaults.
const BUDGETS: Budgets = Budgets::new();

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[WeighError::CODES]);
}

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_list_or_weigh() {
    crumbkit_test::check_grammar(&Cli::command(), &["weigh"], ALLOW);
}

#[test]
fn every_list_command_pages() {
    crumbkit_test::check_listings(&Cli::command(), ALLOW);
}

#[test]
fn every_option_with_many_values_accepts_repeats_and_commas() {
    crumbkit_test::check_filters(&Cli::command(), ALLOW);
}

#[test]
fn every_command_leaf_and_topic_meets_the_guide_rules() {
    crumbkit_test::check_guide(&Cli::command(), &GUIDE);
}

#[test]
fn every_exit_code_meets_the_exit_code_rules() {
    crumbkit_test::check_exit_codes(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    crumbkit_test::check_help_sizes(&Cli::command(), &GUIDE, &BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    crumbkit_test::check_help_goldens(&Cli::command(), &GUIDE, "tests/golden/help");
}

#[test]
fn every_document_is_named_after_the_path_of_the_command_that_prints_it() {
    crumbkit_test::check_document_ids(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    crumbkit_test::check_mounts(&Cli::command(), &GUIDE);
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    crumbkit_test::check_opt_outs(&Cli::command(), ALLOW, &BUDGETS, None, "tests/golden");
}
