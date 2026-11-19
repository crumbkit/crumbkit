//! Tests of the files behind the "Suggest guides from usage" job of cargo
//! crumbkit's skill: `GUIDES.md` holds one row for each finding that
//! `self usage` reports, names only affordances that `AFFORDANCES.md` lists,
//! and `guide-report-v1.json` is the schema whose id the skill names.

use serde_json::Value;

/// cargo crumbkit's skill text.
const SKILL: &str = include_str!("../skill/SKILL.md");

/// The table of guides for each `self usage` finding.
const GUIDES: &str = include_str!("../skill/GUIDES.md");

/// The table of crumbkit's affordances.
const AFFORDANCES: &str = include_str!("../skill/AFFORDANCES.md");

/// The schema of the job's report.
const GUIDE_REPORT: &str = include_str!("../skill/guide-report-v1.json");

/// The schema of `self usage --json`, whose `Kind` lists every finding.
const SELF_USAGE: &str = include_str!("../../crumbkit-usage/schemas/self-usage-v1.json");

/// The headings of the columns of `GUIDES.md`'s table, in order.
const COLUMNS: [&str; 6] = [
    "Finding",
    "Agents were probably trying to",
    "Guide text",
    "Tool change",
    "Affordance",
    "The guide alone is enough when",
];

/// Returns the cells of a Markdown table row, split at each `|` that no
/// backslash escapes, without the empty text before the first bar and after
/// the last.
fn cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = row.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => cells.push(std::mem::take(&mut cell).trim().to_owned()),
            c => cell.push(c),
        }
    }
    // The text before the first bar is empty, and the text after the last
    // bar is left in `cell` and dropped.
    cells.remove(0);
    cells
}

/// Returns the rows of the first table in `text` below its heading and
/// separator lines.
fn table_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(cells)
        .collect()
}

/// Returns the text of each backticked span in `cell`.
fn spans(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

#[test]
fn guides_has_one_row_for_each_finding_that_self_usage_reports_in_its_order() {
    let schema: Value = serde_json::from_str(SELF_USAGE).unwrap();
    let findings: Vec<&str> = schema["$defs"]["Kind"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| kind["const"].as_str().unwrap())
        .collect();
    let rows = table_rows(GUIDES);
    let named: Vec<&str> = rows.iter().map(|row| spans(&row[0])[0]).collect();

    assert_eq!(findings.len(), 9, "{findings:?}");
    assert_eq!(named, findings);
}

#[test]
fn guides_has_the_six_columns_that_the_skill_reads_filled_in_every_row() {
    let header = GUIDES
        .lines()
        .find(|line| line.starts_with('|'))
        .map(cells)
        .unwrap();

    assert_eq!(header, COLUMNS);
    for row in table_rows(GUIDES) {
        assert_eq!(row.len(), COLUMNS.len(), "{row:?}");
        assert!(row.iter().all(|cell| !cell.is_empty()), "{row:?}");
    }
}

#[test]
fn every_affordance_that_guides_names_is_one_that_affordances_lists() {
    for row in table_rows(GUIDES) {
        let named = spans(&row[4]);
        assert!(!named.is_empty(), "{} names no affordance", row[0]);
        for affordance in named {
            assert!(
                AFFORDANCES.contains(&format!("`{affordance}`")),
                "{} names `{affordance}`, which AFFORDANCES.md does not list",
                row[0]
            );
        }
    }
}

#[test]
fn the_guide_report_schema_is_json_with_the_id_that_the_skill_names() {
    let schema: Value = serde_json::from_str(GUIDE_REPORT).unwrap();

    assert_eq!(schema["title"], "cargo-crumbkit/guide-report/v1");
    assert_eq!(
        schema["properties"]["schema"]["const"],
        "cargo-crumbkit/guide-report/v1"
    );
    assert!(SKILL.contains("`cargo-crumbkit/guide-report/v1`"));
    assert!(SKILL.contains("## Suggest guides from usage"));
}

#[test]
fn the_guide_report_requires_every_property_it_declares_and_no_other() {
    let schema: Value = serde_json::from_str(GUIDE_REPORT).unwrap();
    let mut declared: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let mut required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    declared.sort_unstable();
    required.sort_unstable();

    assert_eq!(declared, required);
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn the_skill_reads_self_runs_list_with_full_for_the_fields_its_method_reads() {
    assert!(SKILL.contains("<bin> self runs list --since 14d --all --json --full"));
}

#[test]
fn guides_tells_the_job_what_to_do_with_runs_that_a_program_made() {
    assert!(GUIDES.contains("\n## Program callers\n"));
    assert!(SKILL.contains("\"Program callers\" section of `GUIDES.md`"));
}

#[test]
fn the_guide_report_lists_rejected_findings_beside_the_suggestions() {
    let schema: Value = serde_json::from_str(GUIDE_REPORT).unwrap();
    let rejected = &schema["properties"]["rejected"]["items"];
    let suggestion = &schema["$defs"]["Suggestion"]["properties"];

    assert_eq!(
        rejected["required"],
        serde_json::json!(["command", "findings", "reason"])
    );
    assert_eq!(
        suggestion["made_by"]["enum"],
        serde_json::json!(["agent", "program"])
    );
    assert_eq!(suggestion["findings"]["minItems"], 1);
    assert!(SKILL.contains("- `rejected`:"));
}
