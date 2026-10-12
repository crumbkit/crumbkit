//! Tests for opt-outs and the record of which ones a check used.

use clap::Command;

use super::{Allow, Allowances, Rule};

/// A tool with one resource and one leaf directly under the root.
fn shelf() -> Command {
    Command::new("shelf")
        .subcommand(
            Command::new("books")
                .subcommand(Command::new("list"))
                .subcommand(Command::new("show")),
        )
        .subcommand(Command::new("version"))
}

/// Returns the problems that `allowances` adds for its unused and
/// unmatched opt-outs in `shelf()`.
fn finish(allowances: Allowances<'_>) -> Vec<String> {
    let mut problems = Vec::new();
    allowances.finish(&shelf(), &mut problems);
    problems
}

#[test]
fn an_opt_out_excuses_its_rule_for_the_command_it_names_and_every_command_below_it() {
    const ALLOW: &[Allow] = &[Allow::new(Rule::OneName, "books", "Kept for old scripts.")];
    let mut allowances = Allowances::new(ALLOW, &[Rule::OneName]);

    assert!(allowances.excuses(Rule::OneName, "books"));
    assert!(allowances.excuses(Rule::OneName, "books list"));
    assert!(!allowances.excuses(Rule::OneName, "bookshelves"));
    assert!(!allowances.excuses(Rule::OneName, "version"));
    assert!(!allowances.excuses(Rule::ListPages, "books list"));
}

#[test]
fn an_opt_out_with_an_empty_path_excuses_its_rule_for_the_whole_tree() {
    const ALLOW: &[Allow] = &[Allow::new(Rule::DryRun, "", "The tool's own dry run.")];
    let mut allowances = Allowances::new(ALLOW, &[Rule::DryRun]);

    assert!(allowances.excuses(Rule::DryRun, ""));
    assert!(allowances.excuses(Rule::DryRun, "books list"));
    assert_eq!(finish(allowances), Vec::<String>::new());
}

#[test]
fn an_opt_out_that_names_no_command_is_a_problem() {
    const ALLOW: &[Allow] = &[Allow::new(Rule::RootLeaf, "schema", "Old habit.")];

    assert_eq!(
        finish(Allowances::new(ALLOW, &[Rule::RootLeaf])),
        ["the opt-out from `grammar/root-leaf` for `shelf schema` names no command"]
    );
}

#[test]
fn an_opt_out_that_excuses_no_problem_is_a_problem() {
    const ALLOW: &[Allow] = &[
        Allow::new(Rule::RootLeaf, "version", "Old habit."),
        Allow::new(Rule::RootLeaf, "books", "Old habit."),
    ];
    let mut allowances = Allowances::new(ALLOW, &[Rule::RootLeaf]);
    allowances.excuses(Rule::RootLeaf, "version");

    assert_eq!(
        finish(allowances),
        [
            "the opt-out from `grammar/root-leaf` for `shelf books` excuses no problem, so it is stale"
        ]
    );
}

#[test]
fn a_check_ignores_the_opt_outs_of_rules_it_does_not_own() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::ListPages,
        "nothing here",
        "Not this check's.",
    )];
    let mut allowances = Allowances::new(ALLOW, &[Rule::OneName]);

    assert!(!allowances.excuses(Rule::ListPages, "nothing here"));
    assert_eq!(finish(allowances), Vec::<String>::new());
}
