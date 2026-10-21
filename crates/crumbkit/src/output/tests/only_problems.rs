//! Tests for a listing whose document lists only the rows that report a
//! problem in its default text, and for a row type whose child lines under
//! `--long` differ from its default ones (ADR-0138).

use std::fmt;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use super::finish_with;
use crate::twins::assert_one_word_apart;
use crate::{Document, Listing, Row};

/// A check of one package, with the errors and warnings that it found.
#[derive(Serialize, JsonSchema)]
struct Check {
    /// The package that was checked.
    package: &'static str,
    /// Whether the check passed.
    passed: bool,
    /// The findings of the check, errors first.
    findings: Vec<Finding>,
}

/// One finding of a check.
#[derive(Serialize, JsonSchema)]
struct Finding {
    /// Whether the finding fails the check.
    error: bool,
    /// What the check found.
    text: &'static str,
}

impl Row for Check {
    const COLUMNS: &'static [&'static str] = &["package", "passed"];
    const CHILDREN: Option<&'static str> = Some("findings");
    const WHOLE: bool = true;

    fn is_problem(&self) -> bool {
        !self.passed
    }

    fn child_lines(&self) -> Vec<String> {
        let errors = self.findings.iter().filter(|finding| finding.error);
        errors
            .map(|finding| format!("error: {}", finding.text))
            .collect()
    }

    fn long_child_lines(&self) -> Vec<String> {
        let line = |finding: &Finding| {
            let level = if finding.error { "error" } else { "warning" };
            format!("{level}: {}", finding.text)
        };
        self.findings.iter().map(line).collect()
    }
}

/// The summary line above the table.
#[derive(Serialize, JsonSchema)]
struct Tally {
    /// The number of checks that passed.
    passed: usize,
    /// The number of checks.
    checks: usize,
}

impl fmt::Display for Tally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} of {} checks passed.", self.passed, self.checks)
    }
}

const CHECKS: Document<Listing<Check, Tally>> =
    Document::new("lint", "checks", 1, "The test checks.").only_problems();
const ALL_CHECKS: Document<Listing<Check, Tally>> =
    Document::new("lint", "checks", 1, "The test checks.");

/// Returns a check of `package` with `findings`, each an error flag and a
/// text, which passes when no finding is an error.
fn check(package: &'static str, findings: &[(bool, &'static str)]) -> Check {
    Check {
        package,
        passed: findings.iter().all(|&(error, _)| !error),
        findings: findings
            .iter()
            .map(|&(error, text)| Finding { error, text })
            .collect(),
    }
}

/// Returns a listing of `checks` with a tally of those that passed.
fn tallied(checks: Vec<Check>) -> Listing<Check, Tally> {
    let passed = checks.iter().filter(|check| check.passed).count();
    let total = checks.len();
    Listing::from_all(checks, None)
        .described("checks")
        .summary(Tally {
            passed,
            checks: total,
        })
}

/// Returns three checks, with the one that reports a problem first
/// (OUTPUT-3): one that fails with an error and a warning, one that passes
/// with a warning, and one that passes cleanly.
fn three() -> Listing<Check, Tally> {
    tallied(vec![
        check(
            "web",
            &[(true, "missing licence"), (false, "old lock file")],
        ),
        check("core", &[(false, "slow test")]),
        check("cli", &[]),
    ])
}

#[test]
fn default_text_lists_only_the_rows_that_report_a_problem_under_the_summary_line() {
    let finished = finish_with(&[], CHECKS, three());

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(
        finished.out,
        "2 of 3 checks passed.\nPACKAGE  PASSED\nweb      false\n  error: missing licence\n"
    );
    assert_eq!(
        finished.err,
        "note: 1 of the checks reports a problem, so the command exits with 1.\n"
    );
}

#[test]
fn default_text_without_a_row_that_reports_a_problem_prints_the_summary_line_alone() {
    let listing = tallied(vec![
        check("core", &[(false, "slow test")]),
        check("cli", &[]),
    ]);

    let finished = finish_with(&[], CHECKS, listing);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "2 of 2 checks passed.\n");
    assert_eq!(finished.err, "");
}

#[test]
fn default_text_of_a_listing_without_rows_prints_the_summary_line_and_the_empty_note() {
    let finished = finish_with(&[], CHECKS, tallied(Vec::new()));

    assert_eq!(finished.out, "0 of 0 checks passed.\n");
    assert_eq!(finished.err, "note: no checks match\n");
}

#[test]
fn long_lists_every_row_with_its_long_child_lines() {
    let finished = finish_with(&["--long"], CHECKS, three());

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(
        finished.out,
        "\
2 of 3 checks passed.
PACKAGE  PASSED
web      false
  error: missing licence
  warning: old lock file
core     true
  warning: slow test
cli      true
"
    );
}

#[test]
fn fields_lists_every_row_without_child_lines() {
    let finished = finish_with(&["--fields", "package"], CHECKS, three());

    assert_eq!(
        finished.out,
        "2 of 3 checks passed.\nPACKAGE\nweb\ncore\ncli\n"
    );
}

#[test]
fn json_holds_every_row_as_a_document_that_lists_every_row_prints_it() {
    let only = finish_with(&["--json"], CHECKS, three());
    let all = finish_with(&["--json"], ALL_CHECKS, three());

    assert_eq!(only.code, ExitCode::from(1));
    assert_eq!(only.out, all.out);
    let document: Value = serde_json::from_str(&only.out).unwrap();
    assert_eq!(document["data"].as_array().map(Vec::len), Some(3));
}

#[test]
fn a_document_that_lists_every_row_prints_every_row_in_its_default_text() {
    let finished = finish_with(&[], ALL_CHECKS, three());

    assert_eq!(
        finished.out,
        "\
2 of 3 checks passed.
PACKAGE  PASSED
web      false
  error: missing licence
core     true
cli      true
"
    );
}

#[test]
fn finish_refuses_long_with_json_for_a_whole_row_type_because_json_already_holds_every_field() {
    assert_one_word_apart(&["--json", "--long"], &["--json"]);
    let twin = finish_with(&["--json"], CHECKS, three());
    assert_eq!(twin.code, ExitCode::from(1), "{}", twin.err);

    let finished = finish_with(&["--json", "--long"], CHECKS, three());

    assert_eq!(finished.code, ExitCode::from(2));
    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(
        document["message"],
        "--long adds the long columns of a table, and this command's JSON already holds every field"
    );
    assert_eq!(document["code"], "tool::cli::usage");
    assert_eq!(
        document["recovery"]["text"],
        "drop --long, because --json already prints every field of each row"
    );
}
