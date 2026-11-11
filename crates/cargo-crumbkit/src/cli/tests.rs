//! Tests for cargo-crumbkit's command-line grammar.

use clap::{CommandFactory as _, Parser as _};

use super::{Cli, SKILL, TOOL};

#[test]
fn the_command_tree_with_the_tool_applied_passes_clap_debug_assertions() {
    crumbkit::help::extend(Cli::command(), &TOOL).debug_assert();
}

/// Returns the arguments of `cargo crumbkit weigh` with `args` after it.
fn weigh(args: &[&str]) -> Result<super::WeighArgs, clap::Error> {
    let words = ["cargo", "crumbkit", "weigh", "--bin", "shelf"];
    let cli = Cli::try_parse_from(words.iter().chain(args))?;
    let super::CargoCommand::Crumbkit { command } = cli.cargo;
    match command {
        super::CrumbkitCommand::Weigh(args) => Ok(args),
        other => panic!("weigh parsed as {other:?}"),
    }
}

/// Returns the arguments of `cargo crumbkit docs` with `args` after it.
fn docs(args: &[&str]) -> Result<super::DocsArgs, clap::Error> {
    let words = ["cargo", "crumbkit", "docs"];
    let cli = Cli::try_parse_from(words.iter().chain(args))?;
    let super::CargoCommand::Crumbkit { command } = cli.cargo;
    match command {
        super::CrumbkitCommand::Docs(args) => Ok(args),
        other => panic!("docs parsed as {other:?}"),
    }
}

#[test]
fn docs_needs_a_binary_and_takes_an_optional_file() {
    let args = docs(&["--bin", "shelf"]).unwrap();
    assert_eq!((args.bin.as_str(), args.out), ("shelf", None));
    let args = docs(&["--bin", "shelf", "--out", "commands.md"]).unwrap();
    assert_eq!(args.out, Some("commands.md".into()));
    assert_eq!(
        docs(&[]).unwrap_err().kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn log_implies_usage_and_since_needs_usage_or_log() {
    assert!(!weigh(&[]).unwrap().usage());
    assert!(weigh(&["--usage"]).unwrap().usage());
    assert!(weigh(&["--log", "usage.jsonl"]).unwrap().usage());
    assert_eq!(
        weigh(&["--usage", "--since", "1d"])
            .unwrap()
            .since
            .as_deref(),
        Some("1d")
    );
    assert!(weigh(&["--log", "usage.jsonl", "--since", "1d"]).is_ok());
    assert_eq!(
        weigh(&["--since", "1d"]).unwrap_err().kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

/// Returns the ids in the first column of the table rows under the
/// `### Criteria` heading of the skill's `SKILL.md`.
fn criteria(skill: &str) -> Vec<&str> {
    skill
        .split_once("### Criteria")
        .map(|(_, rest)| rest)
        .unwrap_or_default()
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .filter_map(|line| line.split('|').nth(1))
        .filter_map(|cell| cell.trim().strip_prefix('`')?.strip_suffix('`'))
        .collect()
}

#[test]
fn cargo_crumbkit_s_skill_holds_the_affordances_and_the_report_schema() {
    let names: Vec<&str> = SKILL.get_files().iter().map(|(name, _)| *name).collect();

    assert_eq!(SKILL.name(), "cargo-crumbkit");
    assert_eq!(names, ["AFFORDANCES.md", "audit-report-v1.json"]);
}

#[test]
fn the_audit_report_schema_is_json_with_the_id_that_the_skill_names() {
    let (_, schema) = SKILL.get_files()[1];
    let schema: serde_json::Value = serde_json::from_str(schema).unwrap();

    assert_eq!(schema["title"], "cargo-crumbkit/audit-report/v1");
    assert_eq!(
        schema["properties"]["schema"]["const"],
        "cargo-crumbkit/audit-report/v1"
    );
    assert!(SKILL.text().contains("`cargo-crumbkit/audit-report/v1`"));
}

#[test]
fn every_criterion_that_the_affordances_name_is_a_criterion_of_the_skill() {
    let (_, affordances) = SKILL.get_files()[0];
    let criteria = criteria(SKILL.text());
    // The affordance tables have six columns, so seven bars, and name the
    // criterion in the last.
    let named: Vec<&str> = affordances
        .lines()
        .filter(|line| line.starts_with("| `") && line.matches(" | ").count() == 5)
        .filter_map(|line| line.trim_end_matches('|').rsplit('|').next())
        .filter_map(|cell| cell.trim().strip_prefix('`')?.strip_suffix('`'))
        .collect();

    assert_eq!(criteria.len(), 37, "{criteria:?}");
    assert!(named.len() > 100, "{} affordance rows", named.len());
    for criterion in named {
        assert!(
            criteria.contains(&criterion),
            "{criterion} is no criterion of SKILL.md"
        );
    }
}
