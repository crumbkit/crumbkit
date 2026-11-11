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

/// Returns the cells of a Markdown table row, split at each `|` that no
/// backslash escapes, without the empty text before the first bar and after
/// the last.
fn cells(row: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = row.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cells.last_mut().unwrap().push('|');
                chars.next();
            }
            '|' => cells.push(String::new()),
            c => cells.last_mut().unwrap().push(c),
        }
    }
    cells
        .into_iter()
        .map(|cell| cell.trim().to_owned())
        .filter(|cell| !cell.is_empty())
        .collect()
}

/// Returns the criterion that each row of the affordance tables of
/// `affordances` names in its last column, or `None` for the one row whose
/// criterion is `per feature`. It panics on a row of an affordance table
/// that does not have six cells or whose last cell is neither.
fn named_criteria(affordances: &str) -> Vec<Option<String>> {
    let mut named = Vec::new();
    let mut in_table = false;
    for line in affordances.lines() {
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        let row = cells(line);
        if row.first().map(String::as_str) == Some("Affordance") {
            in_table = true;
            continue;
        }
        if !in_table || row.iter().all(|cell| cell.chars().all(|c| c == '-')) {
            continue;
        }
        assert_eq!(row.len(), 6, "an affordance row without six cells: {line}");
        let last = &row[5];
        if last == "per feature" {
            named.push(None);
            continue;
        }
        let id = last
            .strip_prefix('`')
            .and_then(|cell| cell.strip_suffix('`'))
            .unwrap_or_else(|| panic!("an affordance row names no criterion: {line}"));
        named.push(Some(id.to_owned()));
    }
    named
}

#[test]
fn the_affordances_and_the_skill_name_the_same_criteria() {
    let (_, affordances) = SKILL.get_files()[0];
    let criteria = criteria(SKILL.text());
    let named = named_criteria(affordances);

    assert_eq!(criteria.len(), 37, "{criteria:?}");
    assert_eq!(named.len(), 134, "the affordance rows");
    for criterion in named.iter().flatten() {
        assert!(
            criteria.contains(&criterion.as_str()),
            "{criterion} is no criterion of SKILL.md"
        );
    }
    for criterion in criteria.iter().filter(|id| **id != "crumbkit-gap") {
        assert!(
            named.iter().flatten().any(|id| id == criterion),
            "no affordance row names the criterion {criterion}"
        );
    }
}

#[test]
#[should_panic(expected = "an affordance row without six cells")]
fn an_affordance_row_with_a_missing_cell_fails_the_agreement() {
    named_criteria(
        "| Affordance | Replaces | Smell | Search | Since | Criterion |\n| --- | --- | --- | --- | --- | --- |\n| `Tool` | help | none | 0.1 | `tool-record` |\n",
    );
}
