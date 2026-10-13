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
use crumbkit::Spelling;
use crumbkit_test::{
    Allow, Budgets, Examples, Failing, Pages, PathBudgets, Recoveries, Rule, Twin, Twins,
    Unprovoked,
};
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
    crumbkit_test::check_grammar(
        &Cli::command(),
        &GUIDE,
        &["summarize", "usage", "remove"],
        ALLOW,
    );
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
    crumbkit_test::check_help_sizes(&Cli::command(), Pages::rendered(&GUIDE), &BUDGETS);
}

#[test]
fn every_path_through_the_help_fits_its_weight_budget() {
    crumbkit_test::check_path_weights(&Cli::command(), Pages::rendered(&GUIDE), &PATH_BUDGETS);
}

#[test]
fn every_help_page_matches_its_golden_file() {
    crumbkit_test::check_help_goldens(
        &Cli::command(),
        Pages::rendered(&GUIDE),
        "tests/golden/help",
    );
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
fn every_json_example_cuts_or_refuses_a_result_past_its_output_budget() {
    crumbkit_test::check_output_budget(&Cli::command(), &GUIDE, &EXAMPLES);
}

#[test]
fn every_command_matches_the_checked_in_command_reference() {
    crumbkit_test::check_command_reference(env!("CARGO_BIN_EXE_shelf"), "docs/commands.md");
}

#[test]
fn every_opt_out_matches_the_opt_out_report() {
    crumbkit_test::check_opt_outs(
        &Cli::command(),
        &GUIDE,
        ALLOW,
        &BUDGETS,
        Some(&PATH_BUDGETS),
        "tests/golden",
    );
}

/// `books show` of a book that does not exist.
const SHOW_7: Failing<'_> = Failing::new(&shelf::NOT_FOUND, &["books", "show", "7"]);

/// `books show --from` of an ids file that does not exist.
const MISSING_FILE: Failing<'_> = Failing::new(
    &shelf::UNREADABLE_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/missing.txt"],
);

/// `books show --from` of an ids file with a line that is not an id.
const BAD_LINE: Failing<'_> = Failing::new(
    &shelf::BAD_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/bad-line.txt"],
);

/// `books show --from` of an ids file with 101 ids.
const TOO_MANY: Failing<'_> = Failing::new(
    &shelf::TOO_MANY_IDS,
    &["books", "show", "--from", "tests/fixtures/ids/101-ids.txt"],
);

/// `books show --from` of an empty ids file.
const EMPTY_FILE: Failing<'_> = Failing::new(
    &shelf::EMPTY_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/empty.txt"],
);

/// The words of `books show --from` of an ids file that holds the id 2,
/// which each `--from` line above differs from in the file alone.
const ONE_ID: &[&str] = &["books", "show", "--from", "tests/fixtures/ids/one-id.txt"];

/// The command lines that start shelf's recovery chains. Each `--from`
/// start reads a file under `tests/fixtures/ids/`, and its alternative
/// `shelf books show <ID>...` runs with the id 2.
const STARTS: &[Failing<'_>] = &[
    SHOW_7,
    MISSING_FILE,
    BAD_LINE,
    TOO_MANY,
    EMPTY_FILE,
    Failing::new(
        &crumbkit::codes::CONFIRM_NEEDS_YES,
        &["books", "remove", "2"],
    ),
    Failing::new(
        &crumbkit::codes::CLI_USAGE,
        &["books", "list", "--limit", "101"],
    ),
    Failing::new(&crumbkit::codes::CLI_USAGE, &["books", "show"]),
];

/// shelf's declared codes that no command line in a test provokes, which
/// neither the recovery check nor the twin check needs a command line for.
const UNPROVOKED: &[Unprovoked<'_>] = &[
    Unprovoked::new(
        &shelf::REMOVED_FILE,
        "Only a file of removed books that shelf cannot read or write provokes it, such as one on a full disk.",
    ),
    Unprovoked::new(
        &shelf::NO_STATE_DIRECTORY,
        "The check always sets XDG_STATE_HOME, and only a run without it and without HOME provokes the code.",
    ),
];

/// The settings of shelf's recovery check.
const RECOVERIES: Recoveries<'_> = Recoveries::new(EXAMPLES, Spelling::COLONS, &[shelf::CODES])
    .starts(STARTS)
    .unprovoked(UNPROVOKED)
    .values(&[("ID", "2")]);

#[test]
fn every_recovery_leads_to_a_command_that_runs() {
    crumbkit_test::check_recoveries(&Cli::command(), &GUIDE, &RECOVERIES, "tests/golden");
}

/// The settings of shelf's twin check: one twin for each declared code that
/// a command line can provoke. Each twin differs from its refused line in
/// one word, so that word is the reason for the refusal (TEST-1).
const TWINS: Twins<'_> = Twins::new(EXAMPLES, Spelling::COLONS, &[shelf::CODES])
    .pairs(&[
        Twin::new(SHOW_7, &["books", "show", "2"]),
        Twin::new(MISSING_FILE, ONE_ID),
        Twin::new(BAD_LINE, ONE_ID),
        Twin::new(TOO_MANY, ONE_ID),
        Twin::new(EMPTY_FILE, ONE_ID),
    ])
    .unprovoked(UNPROVOKED);

#[test]
fn every_declared_code_has_a_twin_that_the_tool_accepts() {
    crumbkit_test::check_twins(&Cli::command(), &GUIDE, &TWINS);
}
