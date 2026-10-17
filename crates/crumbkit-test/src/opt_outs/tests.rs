//! Tests for the opt-out report.

use std::fs;

use clap::Command;
use crumbkit::JsonStyle;
use crumbkit::help::Guide;
use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::{Allow, Budget, Budgets, Failing, PathBudgets, Rule};

mod customisations;

/// The opt-outs of the test tool.
const ALLOW: &[Allow] = &[
    Allow::new(
        Rule::RootLeaf,
        "version",
        "People type `shelf version` from habit.",
    ),
    Allow::new(
        Rule::ShowIds,
        "books show",
        "The store looks up one book at a time.",
    ),
];

/// A tool with one resource and one leaf directly under the root.
fn shelf() -> Command {
    Command::new("shelf")
        .subcommand(Command::new("books").subcommand(Command::new("show")))
        .subcommand(Command::new("version"))
}

/// Runs the check on `shelf()` with `allow` and crumbkit's default budgets
/// against `dir` with `action`.
fn run(allow: &[Allow], dir: &TempDir, action: Action) {
    run_with(allow, &Budgets::new(), None, dir, action);
}

/// Runs the check on `shelf()` with `guide` and no opt-outs against `dir`
/// with `action`.
fn run_guided(guide: &Guide, dir: &TempDir, action: Action) {
    check(
        &shelf(),
        guide,
        &[],
        &Budgets::new(),
        None,
        dir.path(),
        &Assert::new().action(action),
    );
}

/// Runs the check on `shelf()` with `allow`, `budgets` and `paths` against
/// `dir` with `action`.
fn run_with(
    allow: &[Allow],
    budgets: &Budgets,
    paths: Option<&PathBudgets<'_>>,
    dir: &TempDir,
    action: Action,
) {
    check(
        &shelf(),
        &Guide::new(),
        allow,
        budgets,
        paths,
        dir.path(),
        &Assert::new().action(action),
    );
}

/// Returns the text of the file `name` in `dir`.
fn read(dir: &TempDir, name: &str) -> String {
    fs::read_to_string(dir.path().join(name)).unwrap()
}

#[test]
fn overwrite_writes_each_opt_out_with_its_reason_and_costs_as_text() {
    let dir = TempDir::new("opt-outs-text").unwrap();

    run(ALLOW, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has 2 opt-outs from the conventions.\n\
         \n\
         grammar/root-leaf: shelf version\n\
         \x20 Reason: People type `shelf version` from habit.\n\
         \x20 Cost: Each such leaf adds a line to the root help, which an agent reads first.\n\
         \x20 Cost: The leaf's name can collide with a resource that the tool adds later.\n\
         \x20 Cost: An agent cannot guess the leaf's siblings from the noun and verb pattern.\n\
         \n\
         listings/show-ids: shelf books show\n\
         \x20 Reason: The store looks up one book at a time.\n\
         \x20 Cost: An agent runs the command once for each id.\n"
    );
}

#[test]
fn overwrite_writes_each_opt_out_as_json() {
    let dir = TempDir::new("opt-outs-json").unwrap();

    run(&ALLOW[1..], &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.json"),
        r#"{
  "schema": "shelf/opt-outs/v1",
  "data": [
    {
      "rule": "listings/show-ids",
      "applies_to": "shelf books show",
      "reason": "The store looks up one book at a time.",
      "costs": [
        "An agent runs the command once for each id."
      ],
      "budget": null
    }
  ]
}
"#
    );
}

#[test]
fn a_tool_without_opt_outs_reports_that_it_has_none() {
    let dir = TempDir::new("opt-outs-none").unwrap();

    run(&[], &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n"
    );
    assert_eq!(
        read(&dir, "opt-outs.json"),
        "{\n  \"schema\": \"shelf/opt-outs/v1\",\n  \"data\": []\n}\n"
    );
}

#[test]
fn files_that_match_the_opt_outs_pass_and_other_files_stay() {
    let dir = TempDir::new("opt-outs-pass").unwrap();
    dir.write("notes.txt", "The report leaves other golden files alone.\n");
    run(ALLOW, &dir, Action::Overwrite);

    run(ALLOW, &dir, Action::Verify);

    assert_eq!(dir.names(), ["notes.txt", "opt-outs.json", "opt-outs.txt"]);
}

#[test]
fn a_missing_and_a_differing_report_each_fail() {
    let dir = TempDir::new("opt-outs-fail").unwrap();
    dir.write(
        "opt-outs.txt",
        "shelf has no opt-outs from the conventions.\n",
    );

    let message = panic_message(|| run(ALLOW, &dir, Action::Verify)).unwrap();

    assert!(
        message.starts_with(
            "the command tree breaks crumbkit's opt-out report rules:\n\
             shelf: the opt-out report differs from the golden file `"
        ),
        "{message}"
    );
    assert!(
        message.contains("opt-outs.json` is missing, and `SNAPSHOTS=overwrite` writes it"),
        "{message}"
    );
}

#[test]
fn the_report_lists_each_budget_that_differs_from_crumbkit_s_default() {
    const BUDGETS: Budgets = Budgets::new()
        .leaf(1_500, "Leaves show long examples.")
        .nodes(&[Budget::new("books", 600, "The group has two verbs.")]);
    let paths = PathBudgets::new("tests/golden/examples").path(4_000, "Paths are short.");
    let dir = TempDir::new("opt-outs-budgets").unwrap();

    run_with(&[], &BUDGETS, Some(&paths), &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has 3 opt-outs from the conventions.\n\
         \n\
         help-budget/leaf: every leaf of shelf\n\
         \x20 Budget: 1500 bytes, where the default is 1200 bytes.\n\
         \x20 Reason: Leaves show long examples.\n\
         \x20 Cost: Each read of the page can cost 300 bytes more than the default, about 75 tokens.\n\
         \n\
         help-budget/command: shelf books\n\
         \x20 Budget: 600 bytes, where the default is 800 bytes.\n\
         \x20 Reason: The group has two verbs.\n\
         \x20 Cost: None, because the budget is at or below the default.\n\
         \n\
         path-budget/path: every path through shelf\n\
         \x20 Budget: 4000 bytes, where the default is 8000 bytes.\n\
         \x20 Reason: Paths are short.\n\
         \x20 Cost: None, because the budget is at or below the default.\n"
    );
}

#[test]
fn the_json_report_gives_a_raised_budget_its_extra_bytes_and_tokens() {
    const BUDGETS: Budgets = Budgets::new();
    const NODES: &[Budget] = &[Budget::new(
        "books show",
        9_001,
        "The page lists every field.",
    )];
    let paths = PathBudgets::new("tests/golden/examples").nodes(NODES);
    let dir = TempDir::new("opt-outs-budget-json").unwrap();

    run_with(&[], &BUDGETS, Some(&paths), &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.json"),
        r#"{
  "schema": "shelf/opt-outs/v1",
  "data": [
    {
      "rule": "path-budget/command",
      "applies_to": "shelf books show",
      "reason": "The page lists every field.",
      "costs": [
        "An agent that finds and runs the command can read 1001 bytes more than the default, about 251 tokens."
      ],
      "budget": {
        "bytes": 9001,
        "default": 8000,
        "extra_bytes": 1001,
        "extra_tokens": 251
      }
    }
  ]
}
"#
    );
}

#[test]
fn the_report_lists_the_output_budget_that_the_guide_sets_beside_crumbkit_s_default() {
    const GUIDE: Guide = Guide::new().output_budget(2_097_152, "Exports print every book.");
    let dir = TempDir::new("opt-outs-output-budget").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has 1 opt-out from the conventions.\n\
         \n\
         output-budget/run: every run of shelf\n\
         \x20 Budget: 2097152 bytes, where the default is 1048576 bytes.\n\
         \x20 Reason: Exports print every book.\n\
         \x20 Cost: A run can print 1048576 bytes more than the default, about 262144 tokens.\n"
    );
}

#[test]
fn a_guide_that_keeps_the_default_output_budget_adds_no_entry() {
    let dir = TempDir::new("opt-outs-default-output-budget").unwrap();

    run_guided(&Guide::new(), &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n"
    );
}

#[test]
fn the_report_lists_the_json_layout_that_the_guide_sets_with_its_cost() {
    const GUIDE: Guide = Guide::new()
        .output_budget(2_097_152, "Exports print every book.")
        .json(JsonStyle::new().pretty("Our scripts read indented JSON."));
    let dir = TempDir::new("opt-outs-json-layout").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    let text = read(&dir, "opt-outs.txt");
    assert!(
        text.starts_with("shelf has 2 opt-outs from the conventions.\n"),
        "{text}"
    );
    assert!(
        text.ends_with(
            "\njson-layout/pretty: every run of shelf\n\
             \x20 Reason: Our scripts read indented JSON.\n\
             \x20 Cost: Every JSON document that is not a stream line is indented by two spaces, which made the documents of the example tool shelf 20 to 62 percent larger, so an agent reads more tokens for each result and fewer rows fit in the output budget.\n"
        ),
        "{text}"
    );
    let json = read(&dir, "opt-outs.json");
    assert!(json.contains("\"rule\": \"json-layout/pretty\""), "{json}");
}

#[test]
fn the_report_lists_a_guide_without_terminal_detection_with_its_reason_and_cost() {
    const GUIDE: Guide =
        Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");
    let dir = TempDir::new("opt-outs-terminal-detection").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    let text = read(&dir, "opt-outs.txt");
    assert_eq!(
        text,
        "shelf has 1 opt-out from the conventions.\n\
         \n\
         terminal-detection/off: every run of shelf\n\
         \x20 Reason: Our CI runs in a pseudo-terminal.\n\
         \x20 Cost: A picker never asks for a missing argument and `Output::confirm` never asks, even at a terminal, so a person must type every argument and `--yes` as a script does.\n\
         \x20 Cost: Progress never prints, and help prints without clap's colours at a terminal.\n"
    );
    let json = read(&dir, "opt-outs.json");
    assert!(
        json.contains("\"rule\": \"terminal-detection/off\""),
        "{json}"
    );
}

#[test]
fn the_report_names_an_example_opt_out_by_its_command_line_with_its_own_costs() {
    const EXAMPLE: &[Allow] = &[Allow::example(
        "books show",
        &["2", "Jane Austen"],
        "The output names the day.",
    )];
    let dir = TempDir::new("opt-outs-example").unwrap();

    run(EXAMPLE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has 1 opt-out from the conventions.\n\
         \n\
         examples/run: shelf books show 2 'Jane Austen'\n\
         \x20 Reason: The output names the day.\n\
         \x20 Cost: The example is not run, so help can show an example that fails.\n\
         \x20 Cost: The example's output is not counted, so the command's output weight is too low when this example prints the most.\n"
    );
}

#[test]
fn the_report_names_an_untwinned_opt_out_by_its_refused_command_line_and_its_cost() {
    const BROKEN: crumbkit::Code = crumbkit::Code::new("shelf", "books", "broken_file");
    const ALLOW: &[Allow] = &[Allow::untwinned(
        Failing::new(&BROKEN, &["books", "show", "--from", "broken file.txt"]),
        "A broken file causes it, and no single word does.",
    )];
    let dir = TempDir::new("opt-outs-untwinned").unwrap();

    run(ALLOW, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has 1 opt-out from the conventions.\n\
         \n\
         twins/no-twin: shelf books show --from 'broken file.txt'\n\
         \x20 Reason: A broken file causes it, and no single word does.\n\
         \x20 Cost: No test shows which word of the command line causes the failure, so a test of the refusal can pass while the command fails for another reason.\n"
    );
}
