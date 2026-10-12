//! The convention tests of shelf, which hold its command tree, guide, help
//! pages and examples to crumbkit's conventions with crumbkit-test.
//!
//! They live in an integration test so that the example check can run the
//! built binary, whose path cargo sets only for integration tests, and so
//! that every check reads the one list of shelf's opt-outs. The example
//! check compares each run's standard output with its golden file under
//! `tests/golden/examples/` (ADR-0005). `SNAPSHOTS=overwrite cargo test`
//! writes, rewrites and deletes the golden files.

use clap::CommandFactory as _;
use crumbkit_test::{Allow, Budgets, Examples, PathBudgets, Rule};
use shelf::{Cli, GUIDE};

/// The reason that the example check skips the examples of `self usage` and
/// `self figures list`.
const DAILY: &str = "The output names the day the report starts, which changes every day, and tests/self_usage.rs checks the output without the day.";

/// shelf's opt-outs from crumbkit's conventions, which every check below
/// reads.
const ALLOW: &[Allow] = &[
    Allow::new(Rule::Examples, "self usage", DAILY),
    Allow::new(Rule::Examples, "self figures list", DAILY),
    Allow::new(
        Rule::PersonalAliases,
        "",
        "A person who runs shelf by hand sees the aliases they wrote on the root's help, and the help checks print it without them.",
    ),
];

/// The budgets of shelf's `-h` pages, which are crumbkit's defaults.
const BUDGETS: Budgets = Budgets::new();

/// The budgets of the paths through shelf's help. shelf's longest path,
/// `commands list`, reads about 3,950 bytes.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples").path(
    4_300,
    "shelf's longest path reads about 3,950 bytes, so 4,300 bytes leaves room for small changes and catches a page or an output that grows by much.",
);

/// shelf's built binary and the directory that holds its example golden
/// files.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_shelf"), "tests/golden/examples").allow(ALLOW);

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_list_show_summarize_usage_or_remove() {
    crumbkit_test::check_grammar(&Cli::command(), &["summarize", "usage", "remove"], ALLOW);
}

#[test]
fn every_command_declares_its_effects() {
    crumbkit_test::check_effects(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_list_command_pages_and_every_show_command_takes_up_to_100_ids() {
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
fn every_path_through_the_help_fits_its_weight_budget() {
    crumbkit_test::check_path_weights(&Cli::command(), &GUIDE, &PATH_BUDGETS);
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
    crumbkit_test::check_document_ids(&Cli::command(), &GUIDE, ALLOW);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    crumbkit_test::check_mounts(&Cli::command(), &GUIDE);
}

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &EXAMPLES);
}

#[test]
fn every_command_matches_the_checked_in_command_reference() {
    crumbkit_test::check_command_reference(env!("CARGO_BIN_EXE_shelf"), "docs/commands.md");
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    crumbkit_test::check_opt_outs(
        &Cli::command(),
        ALLOW,
        &BUDGETS,
        Some(&PATH_BUDGETS),
        "tests/golden",
    );
}
