//! Tests for the opt-out report.

use std::fs;

use clap::Command;
use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::{Allow, Budget, Budgets, PathBudgets, Rule};

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
        "shelf has 2 opt-outs from crumbkit's conventions.\n\
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
  "schema": "crumbkit/opt-outs/v1",
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
        "shelf has no opt-outs from crumbkit's conventions.\n"
    );
    assert_eq!(
        read(&dir, "opt-outs.json"),
        "{\n  \"schema\": \"crumbkit/opt-outs/v1\",\n  \"data\": []\n}\n"
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
        "shelf has no opt-outs from crumbkit's conventions.\n",
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
        "shelf has 3 opt-outs from crumbkit's conventions.\n\
         \n\
         help-budget/leaf: every leaf of shelf\n\
         \x20 Budget: 1500 bytes, where crumbkit's default is 1200 bytes.\n\
         \x20 Reason: Leaves show long examples.\n\
         \x20 Cost: Each read of the page can cost 300 bytes more than crumbkit's default, about 75 tokens.\n\
         \n\
         help-budget/command: shelf books\n\
         \x20 Budget: 600 bytes, where crumbkit's default is 800 bytes.\n\
         \x20 Reason: The group has two verbs.\n\
         \x20 Cost: None, because the budget is at or below crumbkit's default.\n\
         \n\
         path-budget/path: every path through shelf\n\
         \x20 Budget: 4000 bytes, where crumbkit's default is 8000 bytes.\n\
         \x20 Reason: Paths are short.\n\
         \x20 Cost: None, because the budget is at or below crumbkit's default.\n"
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
  "schema": "crumbkit/opt-outs/v1",
  "data": [
    {
      "rule": "path-budget/command",
      "applies_to": "shelf books show",
      "reason": "The page lists every field.",
      "costs": [
        "An agent that finds and runs the command can read 1001 bytes more than crumbkit's default, about 251 tokens."
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
