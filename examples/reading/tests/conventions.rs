//! The convention tests of reading, which hold its command tree, guide, help
//! pages and examples to crumbkit's conventions with crumbkit-test.
//!
//! Each check reads the command tree and the guide that reading builds from
//! the lists in `tests/fixtures/lists/`, so the checks cover the commands
//! that reading adds at run time. They live in an integration test so that
//! the example check can run the built binary, whose path cargo sets only
//! for integration tests. The example check runs each example in a new copy
//! of the fixture directory, with `READING_DIR` naming the copy, and
//! compares each run's standard output with its golden file under
//! `tests/golden/examples/` (ADR-0005). `SNAPSHOTS=overwrite cargo test` writes, rewrites and
//! deletes the golden files.

use std::path::Path;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit_test::{
    Allow, Budgets, Examples, Failing, Pages, PathBudgets, Recoveries, Rule, Twin, Twins,
    Unprovoked,
};

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
    4_300,
    "reading's longest path, commands list, reads about 4,050 bytes, so 4,300 bytes leaves room for small changes and catches a page or an output that grows by much.",
);

/// reading's built binary, the directory that holds its example golden
/// files, and the fixture lists that each run reads. Each run starts in a
/// new copy of the fixture lists, and `READING_DIR` names that copy, so an
/// example of `entries set` that writes to a list leaves the fixture as it
/// is and the next run sees the list unchanged. The copy is why the run
/// provides `writable-lists`. The fixture holds no list that is not UTF-8,
/// so the run does not provide `scanned-list`, and the check skips the
/// example of `entries export` that needs it.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_reading"), "tests/golden/examples")
        .fresh_dir(LISTS)
        .env(&[("READING_DIR", ".")])
        .provides(&["writable-lists"])
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
    crumbkit_test::check_names(&command, Pages::rendered(&guide), ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_list_show_stream_edit_set_or_export() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_grammar(
        &command,
        Pages::rendered(&guide),
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
    crumbkit_test::check_guide(&command, &guide, ALLOW);
}

#[test]
fn every_exit_code_meets_the_exit_code_rules() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_exit_codes(&command, &guide, ALLOW);
}

#[test]
fn every_help_page_fits_its_size_budget() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_help_sizes(&command, Pages::binary(&guide, &EXAMPLES), &BUDGETS);
}

#[test]
fn every_path_through_the_help_fits_its_weight_budget() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_path_weights(&command, Pages::binary(&guide, &EXAMPLES), &PATH_BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_help_goldens(
        &command,
        Pages::binary(&guide, &EXAMPLES),
        "tests/golden/help",
    );
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

/// `entries list` of a list that does not exist.
const UNREAD: Failing<'_> = Failing::new(
    &reading::NOT_FOUND,
    &["entries", "list", "--list", "unread"],
);

/// A dry run of `entries set` at a position past the end of the list.
const POSITION_7: Failing<'_> = Failing::new(
    &reading::OUT_OF_RANGE,
    &[
        "entries",
        "set",
        "--list",
        "to-read",
        "--position",
        "7",
        "--dry-run",
        "Emma",
    ],
);

/// reading's declared code that no command line in a test provokes, which
/// neither the recovery check nor the twin check needs a command line for.
const UNPROVOKED: &[Unprovoked<'_>] = &[Unprovoked::new(
    &reading::UNREADABLE,
    "Only a list file that exists and that reading cannot read provokes it, and a file's permissions differ between platforms.",
)];

/// The settings of reading's recovery check. A missing list, a position
/// past the end of a list and a version conflict each start a chain. Each
/// chain runs in its own copy of the fixture lists, and the last two run on
/// a dry run, so they write nothing to that copy either.
const RECOVERIES: Recoveries<'_> = Recoveries::new(EXAMPLES, &[reading::CODES])
    .starts(&[
        UNREAD,
        POSITION_7,
        Failing::new(
            &crumbkit::codes::VERSION_CONFLICT,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "2",
                "--expect",
                "Emma",
                "--dry-run",
                "Mary Barton",
            ],
        ),
    ])
    .unprovoked(UNPROVOKED);

#[test]
fn every_recovery_leads_to_a_command_that_runs() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_recoveries(&command, &guide, &RECOVERIES, "tests/golden");
}

/// The settings of reading's twin check: one twin for each declared code
/// that a command line can provoke. Each twin differs from its refused line
/// in one word, so that word is the reason for the refusal (TEST-1).
const TWINS: Twins<'_> = Twins::new(EXAMPLES, &[reading::CODES])
    .pairs(&[
        Twin::new(UNREAD, &["entries", "list", "--list", "to-read"]),
        Twin::new(
            POSITION_7,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "1",
                "--dry-run",
                "Emma",
            ],
        ),
    ])
    .unprovoked(UNPROVOKED);

#[test]
fn every_declared_code_has_a_twin_that_the_tool_accepts() {
    let (command, guide) = fixture_tool();
    crumbkit_test::check_twins(&command, &guide, &TWINS);
}
