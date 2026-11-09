//! Tests for cargo-crumbkit as a whole, which hold it to the conventions it
//! measures in other tools.

use clap::CommandFactory as _;
use crumbkit_test::{Budgets, Pages};

use crate::cli::{ALLOW, Cli, TOOL};

/// The budgets of cargo-crumbkit's `-h` pages, which are crumbkit's
/// defaults.
const BUDGETS: Budgets = Budgets::new();

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), Pages::rendered(&TOOL), ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_list_weigh_or_docs() {
    crumbkit_test::check_grammar(
        &Cli::command(),
        Pages::rendered(&TOOL),
        &["weigh", "docs"],
        ALLOW,
    );
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
fn every_command_leaf_and_topic_meets_the_help_rules() {
    crumbkit_test::check_help(&Cli::command(), &TOOL, ALLOW);
}

#[test]
fn every_exit_code_meets_the_exit_code_rules() {
    crumbkit_test::check_exit_codes(&Cli::command(), &TOOL, ALLOW);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    crumbkit_test::check_help_sizes(&Cli::command(), Pages::rendered(&TOOL), &BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    crumbkit_test::check_help_goldens(&Cli::command(), Pages::rendered(&TOOL), "tests/golden/help");
}

#[test]
fn every_document_is_named_after_the_path_of_the_command_that_prints_it() {
    crumbkit_test::check_document_ids(&Cli::command(), &TOOL, ALLOW);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    crumbkit_test::check_mounts(&Cli::command(), &TOOL);
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    crumbkit_test::check_opt_outs(
        &Cli::command(),
        &TOOL,
        ALLOW,
        &BUDGETS,
        None,
        "tests/golden",
    );
}
