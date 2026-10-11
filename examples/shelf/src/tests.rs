//! Tests for shelf as a whole.

use clap::CommandFactory as _;
use crumbkit_test::{Budgets, PathBudgets};

use crate::SPELLING;
use crate::books::BooksError;
use crate::cli::{Cli, GUIDE};

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[BooksError::CODES]);
}

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), &[]);
}

#[test]
fn every_command_is_a_resource_followed_by_list_show_or_usage() {
    crumbkit_test::check_grammar(&Cli::command(), &["usage"], &[]);
}

#[test]
fn every_list_command_pages_and_every_show_command_takes_up_to_100_ids() {
    crumbkit_test::check_listings(&Cli::command(), &[]);
}

#[test]
fn every_option_with_many_values_accepts_repeats_and_commas() {
    crumbkit_test::check_filters(&Cli::command(), &[]);
}

#[test]
fn every_command_leaf_and_topic_meets_the_guide_rules() {
    crumbkit_test::check_guide(&Cli::command(), &GUIDE);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    crumbkit_test::check_help_sizes(&Cli::command(), &GUIDE, &Budgets::new());
}

/// shelf's longest path, `commands list`, reads about 3,500 bytes, so a
/// budget of 4,000 bytes leaves room for small changes and catches a page
/// or an output that grows by much.
#[test]
fn every_path_through_the_help_fits_its_weight_budget() {
    crumbkit_test::check_path_weights(
        &Cli::command(),
        &GUIDE,
        &PathBudgets::new("tests/golden/examples").path(4_000),
    );
}

#[test]
fn every_help_page_matches_its_golden_file() {
    crumbkit_test::check_help_goldens(&Cli::command(), &GUIDE, "tests/golden/help");
}

#[test]
fn every_document_schema_matches_its_golden_file() {
    crumbkit_test::check_schema_goldens(&Cli::command(), &GUIDE, "tests/golden/schemas");
}

#[test]
fn every_document_is_named_after_the_path_of_the_command_that_prints_it() {
    crumbkit_test::check_document_ids(&Cli::command(), &GUIDE);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    crumbkit_test::check_mounts(&Cli::command(), &GUIDE);
}
