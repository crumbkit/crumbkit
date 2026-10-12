//! The convention tests of reading, which hold its command tree, guide, help
//! pages and examples to crumbkit's conventions with crumbkit-test.
//!
//! Each check reads the command tree and the guide that reading builds from
//! the lists in `tests/fixtures/lists/`, so the checks cover the commands
//! that reading adds at run time. They live in an integration test so that
//! the example check can run the built binary, whose path cargo sets only
//! for integration tests. The example check runs each example with
//! `READING_DIR` set to the fixture directory, and compares each run's
//! standard output with its golden file under `tests/golden/examples/`
//! (ADR-0005). `SNAPSHOTS=overwrite cargo test` writes, rewrites and
//! deletes the golden files.

use std::path::Path;

use clap::Command;
use crumbkit::Spelling;
use crumbkit::help::Guide;
use crumbkit_test::{Allow, Budgets, Examples, Failing, PathBudgets, Recoveries, Rule, Unprovoked};

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// The reason that the examples of each list's `edit` are not run.
const EDITOR: &str = "edit hands the terminal to the caller's editor, which waits for a person and prints no output to compare";

/// The reason that each list's `edit` has no `--yes`.
const SAVE: &str = "the person in the editor chooses whether to save the list, so there is nothing for --yes to confirm";

/// The reason that each list's `edit` has no `--expect`.
const WHOLE_FILE: &str = "the editor saves the whole file, and the person in the editor sees the list as it is when the editor opens it";

/// reading's opt-outs from crumbkit's conventions, which every check below
/// reads. They name the `edit` command of each fixture list.
const ALLOW: &[Allow] = &[
    Allow::new(Rule::Examples, "finished edit", EDITOR),
    Allow::new(Rule::Examples, "to-read edit", EDITOR),
    Allow::new(Rule::Yes, "finished edit", SAVE),
    Allow::new(Rule::Yes, "to-read edit", SAVE),
    Allow::new(Rule::Expect, "finished edit", WHOLE_FILE),
    Allow::new(Rule::Expect, "to-read edit", WHOLE_FILE),
];

/// The budgets of reading's `-h` pages, which are crumbkit's defaults.
const BUDGETS: Budgets = Budgets::new();

/// The budgets of the paths through reading's help.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples").path(
    3_900,
    "reading's longest path, commands list, reads about 3,600 bytes, so 3,900 bytes leaves room for small changes and catches a page or an output that grows by much.",
);

/// reading's built binary, the directory that holds its example golden
/// files, and the fixture lists that each run reads.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_reading"), "tests/golden/examples")
        .env(&[("READING_DIR", LISTS)])
        .allow(ALLOW);

/// Returns the command tree and the guide that reading builds from the lists
/// in its fixture directory.
fn fixture_tool() -> (Command, Guide) {
    let lists = reading::lists_in(Path::new(LISTS));
    (reading::command(&lists), reading::guide(&lists))
}

#[test]
fn every_command_and_argument_has_one_name() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_names(&command, &guide, ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_list_show_stream_edit_set_or_export() {
    crumbkit_test::check_grammar(
        &fixture_tool().0,
        &["stream", "edit", "set", "export"],
        ALLOW,
    );
}

#[test]
fn every_command_declares_its_effects() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_effects(&command, &guide, ALLOW);
}

#[test]
fn every_list_command_pages() {
    crumbkit_test::check_listings(&fixture_tool().0, ALLOW);
}

#[test]
fn every_option_with_many_values_accepts_repeats_and_commas() {
    crumbkit_test::check_filters(&fixture_tool().0, ALLOW);
}

#[test]
fn every_command_and_leaf_meets_the_guide_rules() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_guide(&command, &guide);
}

#[test]
fn every_exit_code_meets_the_exit_code_rules() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_exit_codes(&command, &guide, ALLOW);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_help_sizes(&command, &guide, &BUDGETS);
}

#[test]
fn every_path_through_the_help_fits_its_weight_budget() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_path_weights(&command, &guide, &PATH_BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_help_goldens(&command, &guide, "tests/golden/help");
}

#[test]
fn every_document_schema_matches_its_golden_file() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_schema_goldens(&command, &guide, "tests/golden/schemas");
}

#[test]
fn every_document_is_named_after_the_path_of_the_command_that_prints_it() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_document_ids(&command, &guide, ALLOW);
}

#[test]
fn every_crumbkit_command_sits_at_its_fixed_path() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_mounts(&command, &guide);
}

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_examples(&command, &guide, &EXAMPLES);
}

#[test]
fn every_json_example_cuts_or_refuses_a_result_past_its_output_budget() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_output_budget(&command, &guide, &EXAMPLES);
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_opt_outs(
        &command,
        &guide,
        ALLOW,
        &BUDGETS,
        Some(&PATH_BUDGETS),
        "tests/golden",
    );
}

/// The settings of reading's recovery check. A missing list, a position
/// past the end of a list and a version conflict each start a chain. The
/// last two run on a dry run, so the check writes no fixture list.
const RECOVERIES: Recoveries<'_> = Recoveries::new(EXAMPLES, Spelling::COLONS, &[reading::CODES])
    .starts(&[
        Failing::new(&reading::NOT_FOUND, &["entries", "list", "--list", "unread"]),
        Failing::new(
            &reading::OUT_OF_RANGE,
            &["entries", "set", "--list", "to-read", "--position", "7", "--dry-run", "Emma"],
        ),
        Failing::new(
            &crumbkit::codes::VERSION_CONFLICT,
            &[
                "entries", "set", "--list", "to-read", "--position", "2", "--expect", "Emma",
                "--dry-run", "Mary Barton",
            ],
        ),
    ])
    .unprovoked(&[Unprovoked::new(
        &reading::UNREADABLE,
        "Only a list file that exists and that reading cannot read provokes it, and a file's permissions differ between platforms.",
    )]);

#[test]
fn every_recovery_leads_to_a_command_that_runs() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_recoveries(&command, &guide, &RECOVERIES, "tests/golden");
}
