//! Tests for a listing whose document lists only the rows that report a
//! problem in its default text, and for a row type whose child lines under
//! `--long` differ from its default ones (ADR-0138).

use std::fmt;
use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use clap::CommandFactory as _;

use super::{Tool, built_with, finish_output, finish_with};
use crate::help::Guide;
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
fn long_with_json_for_a_whole_row_type_prints_the_json_document_and_a_note() {
    let json = finish_with(&["--json"], CHECKS, three());

    let finished = finish_with(&["--json", "--long"], CHECKS, three());

    assert_eq!(finished.code, ExitCode::from(1), "{}", finished.err);
    assert_eq!(finished.out, json.out);
    assert_eq!(json.err, "");
    assert_eq!(
        finished.err,
        "note: --long adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}

/// A link from a pointer to its target, which prints its own text line,
/// has a long column and does not set `Row::WHOLE`.
#[derive(Serialize, JsonSchema)]
struct Link {
    /// The name that points.
    pointer: &'static str,
    /// Whether the target exists.
    found: bool,
    /// What the pointer points at.
    target: &'static str,
}

impl Row for Link {
    const COLUMNS: &'static [&'static str] = &["pointer", "found"];
    const LONG_COLUMNS: &'static [&'static str] = &["target"];
    const LINE: Option<&'static str> = Some("<POINTER> -> <TARGET>");

    fn is_problem(&self) -> bool {
        !self.found
    }

    fn text_line(&self) -> Option<String> {
        Some(format!("{} -> {}", self.pointer, self.target))
    }
}

const LINKS: Document<Listing<Link>> =
    Document::new("links", "links", 1, "The test links.").only_problems();

/// Returns two links, the broken one first (OUTPUT-3).
fn links() -> Listing<Link> {
    let broken = Link {
        pointer: "docs",
        found: false,
        target: "gone",
    };
    let working = Link {
        pointer: "home",
        found: true,
        target: "index",
    };
    Listing::from_all(vec![broken, working], None)
}

#[test]
fn rows_with_their_own_text_line_print_only_the_lines_of_rows_that_report_a_problem() {
    let finished = finish_with(&[], LINKS, links());

    assert_eq!(finished.out, "docs -> gone\n");
}

#[test]
fn finish_accepts_long_with_json_for_a_row_type_that_does_not_print_whole() {
    let finished = finish_with(&["--json", "--long"], LINKS, links());

    assert_eq!(finished.code, ExitCode::from(1), "{}", finished.err);
    let document: Value = serde_json::from_str(&finished.out).unwrap();
    assert_eq!(document["data"][1]["target"], "index");
}

#[test]
fn the_footer_of_a_page_counts_every_row_of_the_listing_under_the_filter() {
    let page = Listing::new(
        vec![
            check("web", &[(true, "missing licence")]),
            check("cli", &[]),
        ],
        5,
    )
    .described("checks");

    let finished = finish_with(
        &[],
        CHECKS,
        page.summary(Tally {
            passed: 4,
            checks: 5,
        }),
    );

    assert_eq!(
        finished.out,
        "4 of 5 checks passed.\nPACKAGE  PASSED\nweb      false\n  error: missing licence\n"
    );
    assert!(
        finished
            .err
            .starts_with("Showing 2 of 5 checks. --all shows every one.\n"),
        "{}",
        finished.err
    );
}

#[test]
fn the_budget_cut_line_counts_the_printed_problem_rows_against_the_listing_total() {
    let failing = (0..10).map(|_| check("web", &[(true, "missing licence")]));
    let passing = (0..10).map(|_| check("cli", &[]));
    let listing = tallied(failing.chain(passing).collect());
    let guide = Guide::new().output_budget(160, "the tests print little");
    let output = built_with(Tool::command(), &guide, &["tool", "list"]);

    let finished = finish_output(output, CHECKS, listing);

    let rows = "web      false\n  error: missing licence\n".repeat(3);
    assert_eq!(
        finished.out,
        format!("10 of 20 checks passed.\nPACKAGE  PASSED\n{rows}")
    );
    assert_eq!(
        finished.err,
        "Showing 3 of 20 checks, because output stops at 160 bytes. \
         Ask for less with --limit or --fields.\n\
         note: 10 of the checks report a problem, so the command exits with 1.\n"
    );
}

/// Whether a service is down, which reads `ok` or `down` in text and is a
/// boolean in JSON, so the problem field is `false` for a healthy service.
#[derive(Clone, Copy)]
struct Fault(bool);

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0 { "down" } else { "ok" })
    }
}

impl crate::Shown for Fault {
    fn json<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.0)
    }

    fn schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        bool::json_schema(generator)
    }
}

crate::shown!(Fault);

/// A service whose problem field is a shown value.
#[derive(Serialize, JsonSchema)]
struct Service {
    /// The service's name.
    name: &'static str,
    /// Whether the service is down.
    fault: Fault,
}

impl Row for Service {
    const COLUMNS: &'static [&'static str] = &["name", "fault"];
    const PROBLEM: Option<&'static str> = Some("fault");
}

const SERVICES: Document<Listing<Service>> =
    Document::new("lint", "services", 1, "The test services.").only_problems();

#[test]
fn a_shown_problem_field_is_judged_by_its_json_form_and_printed_as_its_text() {
    // The healthy service comes first, against the usual order, so a filter
    // that judged the text form `ok` as a problem would print it.
    let services = vec![
        Service {
            name: "web",
            fault: Fault(false),
        },
        Service {
            name: "db",
            fault: Fault(true),
        },
    ];

    let finished = finish_with(&[], SERVICES, Listing::from_all(services, None));

    assert_eq!(finished.code, ExitCode::from(1));
    assert_eq!(finished.out, "NAME  FAULT\ndb    down\n");
}
