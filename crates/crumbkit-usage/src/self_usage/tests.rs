use std::io::Cursor;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use clap::Parser;
use serde_json::json;

use super::{CallerRuns, CodeRuns, FindingRow, SelfUsage, UsageSummary, report_from};
use crate::testing::{line, now, tool};
use crumbkit::Rows;
use crumbkit::help::{Exit, Guide};

/// Returns the summary of `runs` runs by people, in `sessions` sessions,
/// since 30 September 2026, with `skipped` skipped records.
fn summary(runs: u64, sessions: u64, skipped: u64) -> UsageSummary {
    UsageSummary {
        runs,
        sessions,
        callers: vec![CallerRuns {
            caller: "person".to_owned(),
            runs,
        }],
        since: "2026-09-30".to_owned(),
        skipped,
        tolerated: Vec::new(),
    }
}

#[test]
fn the_summary_line_counts_runs_sessions_and_callers_with_thousands_separators() {
    let mut summary = summary(1_284, 23, 0);
    summary.callers = vec![
        CallerRuns {
            caller: "claude-code".to_owned(),
            runs: 1_201,
        },
        CallerRuns {
            caller: "person".to_owned(),
            runs: 83,
        },
    ];

    assert_eq!(
        summary.to_string(),
        "1,284 runs in 23 sessions since 30 Sep: claude-code 1,201, person 83."
    );
}

#[test]
fn the_summary_line_names_one_run_and_one_session_in_the_singular() {
    assert_eq!(
        summary(1, 1, 0).to_string(),
        "1 run in 1 session since 30 Sep: person 1."
    );
}

#[test]
fn the_summary_line_of_a_log_without_runs_names_only_the_day() {
    let mut summary = summary(0, 0, 0);
    summary.callers.clear();

    assert_eq!(summary.to_string(), "0 runs since 30 Sep.");
}

#[test]
fn the_summary_line_says_how_many_records_were_skipped() {
    assert_eq!(
        summary(2, 1, 1_002).to_string(),
        "2 runs in 1 session since 30 Sep: person 2. 1,002 records were skipped because they name a command, flag, code or caller that the tool does not write."
    );
}

#[test]
fn the_summary_line_ends_with_the_codes_tolerated_most_often() {
    let mut summary = summary(2, 1, 0);
    summary.tolerated = vec![
        CodeRuns {
            code: "shelf::catalogue::locked".to_owned(),
            runs: 1_200,
        },
        CodeRuns {
            code: "shelf::network::timed_out".to_owned(),
            runs: 1,
        },
    ];

    assert_eq!(
        summary.to_string(),
        "2 runs in 1 session since 30 Sep: person 2. Tolerated most often: shelf::catalogue::locked 1,200, shelf::network::timed_out 1."
    );
}

#[test]
fn the_summary_lists_the_ten_codes_tolerated_by_the_most_runs() {
    let log: String = (0..12_u64)
        .map(|index| {
            let codes: Vec<String> = (0..=index.min(9))
                .map(|code| format!("shelf::books::c{code}"))
                .chain((index >= 10).then(|| format!("shelf::books::rare{index}")))
                .collect();
            line(index + 1, "books list", &json!({ "tolerated": codes }))
        })
        .collect();
    let listing = report_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(60),
        now(),
    );

    let summary = serde_json::to_value(Rows::summary(&listing)).unwrap();

    let tolerated = summary["tolerated"].as_array().unwrap();
    assert_eq!(tolerated.len(), 10);
    assert_eq!(
        tolerated[0],
        json!({"code": "shelf::books::c0", "runs": 12})
    );
    assert_eq!(tolerated[9], json!({"code": "shelf::books::c9", "runs": 3}));
}

#[test]
fn the_summary_serializes_its_figures_as_fields() {
    assert_eq!(
        serde_json::to_value(summary(2, 1, 3)).unwrap(),
        json!({
            "runs": 2,
            "sessions": 1,
            "callers": [{"caller": "person", "runs": 2}],
            "since": "2026-09-30",
            "skipped": 3,
            "tolerated": [],
        })
    );
}

/// Returns the finding, runs, command and suggestion of each row of the
/// report of `log` over the last 7 days.
fn rows(log: &str) -> Vec<(String, u64, String, String)> {
    let listing = report_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(7 * 86_400),
        now(),
    );
    listing
        .rows()
        .iter()
        .map(|row: &FindingRow| {
            let value = serde_json::to_value(row).unwrap();
            (
                value["finding"].as_str().unwrap().to_owned(),
                row.runs,
                row.command.clone(),
                row.suggestion.clone(),
            )
        })
        .collect()
}

#[test]
fn each_finding_prints_with_its_command_and_a_suggestion() {
    let one_id = json!({"ids": 1});
    let refused = json!({"exit": 2, "nearest": "--all", "out_bytes": null});
    let failed = json!({"exit": 1, "code": "shelf::books::not_found"});
    let log = [
        line(300, "books show", &one_id),
        line(290, "books show", &one_id),
        line(280, "books show", &one_id),
        line(200, "books list", &refused),
        line(100, "books show", &failed),
        line(90, "books show", &json!({})),
        line(50, "", &json!({"ms": 1_500})),
    ]
    .concat();

    assert_eq!(
        rows(&log),
        [
            (
                "one id at a time".to_owned(),
                3,
                "books show".to_owned(),
                "pass every id to one call".to_owned()
            ),
            (
                "usage error".to_owned(),
                1,
                "books list".to_owned(),
                "1 of the runs typed a flag close to --all".to_owned()
            ),
            (
                "retry after failure".to_owned(),
                1,
                "books show".to_owned(),
                "shelf::books::not_found failed and the command ran again within 60 s".to_owned()
            ),
            (
                "slow command".to_owned(),
                1,
                "shelf".to_owned(),
                "the 95th percentile of its wall time is 1,500 ms".to_owned()
            ),
        ]
    );
}

#[test]
fn a_finding_row_carries_the_figures_of_its_command() {
    let refused = json!({"exit": 2, "out_bytes": null, "flags": ["--full"]});
    let log = [
        line(30, "books list", &refused),
        line(
            20,
            "books list",
            &json!({"flags": ["--all"], "out_bytes": 25_000}),
        ),
        line(10, "books list", &json!({"help": "short"})),
    ]
    .concat();
    let listing = report_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(60),
        now(),
    );

    let row = serde_json::to_value(&listing.rows()[0]).unwrap();

    assert_eq!(row["finding"], "usage error");
    assert_eq!(
        row["figures"],
        json!({
            "runs": 2,
            "help_pages": 1,
            "help_before": 0,
            "refused": 1,
            "failed": 0,
            "p95_ms": 10,
            "p95_out_bytes": 25_000,
            "typed_full": 1,
            "typed_all": 1,
            "typed_long": 0,
            "exits": [],
        })
    );
}

#[test]
fn a_finding_row_counts_each_exit_from_3_with_the_meaning_that_the_guide_declares_for_it() {
    const GUIDE: Guide = Guide::new().exits(&[
        Exit::new(3, "The catalogue was locked."),
        Exit::new(124, "The import ran out of time.").commands(&[&["books", "show"]]),
    ]);
    let log = [
        line(40, "books list", &json!({"exit": 2})),
        line(30, "books list", &json!({"exit": 124})),
        line(20, "books list", &json!({"exit": 3})),
        line(10, "books list", &json!({"exit": 3})),
    ]
    .concat();
    let listing = report_from(
        &tool(),
        &GUIDE,
        Some(Cursor::new(log)),
        Duration::from_secs(60),
        now(),
    );

    let row = serde_json::to_value(&listing.rows()[0]).unwrap();

    assert_eq!(
        row["figures"]["exits"],
        json!([
            {"exit": 3, "meaning": "The catalogue was locked.", "runs": 2},
            {"exit": 124, "meaning": null, "runs": 1},
        ])
    );
}

#[test]
fn a_finding_row_gives_an_exit_inside_a_declared_range_the_meaning_of_the_range() {
    const GUIDE: Guide =
        Guide::new().exits(&[Exit::range(64, 78, "The importer ended with its own code.")]);
    let log = [
        line(30, "books list", &json!({"exit": 2})),
        line(20, "books list", &json!({"exit": 70})),
        line(10, "books list", &json!({"exit": 79})),
    ]
    .concat();
    let listing = report_from(
        &tool(),
        &GUIDE,
        Some(Cursor::new(log)),
        Duration::from_secs(60),
        now(),
    );

    let row = serde_json::to_value(&listing.rows()[0]).unwrap();

    assert_eq!(
        row["figures"]["exits"],
        json!([
            {"exit": 70, "meaning": "The importer ended with its own code.", "runs": 1},
            {"exit": 79, "meaning": null, "runs": 1},
        ])
    );
}

#[test]
fn the_report_reads_the_runs_since_the_duration_before_now_and_names_that_day() {
    let log = [
        line(3 * 86_400, "books list", &json!({})),
        line(60, "books list", &json!({})),
    ]
    .concat();

    let listing = report_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(86_400),
        now(),
    );

    let summary = Rows::summary(&listing).unwrap();
    assert_eq!(summary.runs, 1);
    assert_eq!(summary.since, "2026-10-10");
}

#[test]
fn without_a_log_the_report_has_no_runs_and_no_findings() {
    let listing = report_from(
        &tool(),
        &Guide::new(),
        None::<Cursor<&str>>,
        Duration::from_secs(60),
        now(),
    );

    assert_eq!(listing.rows().len(), 0);
    assert_eq!(
        Rows::summary(&listing).unwrap().to_string(),
        "0 runs since 11 Oct."
    );
}

#[test]
fn a_clock_past_the_year_9999_reports_the_last_day_that_humantime_formats() {
    let far = SystemTime::UNIX_EPOCH + Duration::from_secs(300_000_000_000);

    let listing = report_from(
        &tool(),
        &Guide::new(),
        None::<Cursor<&str>>,
        Duration::from_secs(60),
        far,
    );

    assert_eq!(Rows::summary(&listing).unwrap().since, "9999-12-31");
}

/// A tool whose only arguments are `self usage`'s.
#[derive(Parser)]
struct Tool {
    #[command(flatten)]
    args: SelfUsage,
}

#[test]
fn since_takes_a_humantime_duration_and_defaults_to_7_days() {
    let default = Tool::try_parse_from(["tool"]).unwrap();
    let twelve_hours = Tool::try_parse_from(["tool", "--since", "12h"]).unwrap();
    let refused = Tool::try_parse_from(["tool", "--since", "yesterday"]);

    assert_eq!(default.args.since, Duration::from_secs(7 * 86_400));
    assert_eq!(twelve_hours.args.since, Duration::from_secs(12 * 3_600));
    assert!(refused.is_err());
}

#[test]
fn log_names_a_file_to_read_in_place_of_the_tool_s_own_log() {
    let default = Tool::try_parse_from(["tool"]).unwrap();
    let named = Tool::try_parse_from(["tool", "--log", "copy.jsonl"]).unwrap();

    assert_eq!(default.args.log, None);
    assert_eq!(named.args.log, Some(PathBuf::from("copy.jsonl")));
}

#[test]
fn the_checked_in_schema_matches_the_usage_report_document_schema() {
    let generated = crumbkit::schemas::document_schema(super::SELF_USAGE);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/self-usage-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/self-usage-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}

#[test]
fn a_since_that_reaches_before_1970_reads_the_whole_log_from_1970() {
    let log = line(100, "books list", &json!({}));

    let listing = report_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(100 * 365 * 86_400),
        now(),
    );

    let summary = Rows::summary(&listing).unwrap();
    assert_eq!(summary.runs, 1);
    assert_eq!(summary.since, "1970-01-01");
}

#[test]
fn a_subcommand_that_takes_a_usage_report_without_a_doc_comment_has_no_description() {
    use clap::{Args as _, Command};

    let leaves = [
        SelfUsage::augment_args(Command::new("usage")),
        crate::SelfFigures::augment_args(Command::new("figures")),
    ];

    for leaf in &leaves {
        assert_eq!(leaf.get_about(), None, "{}", leaf.get_name());
        assert_eq!(leaf.get_long_about(), None, "{}", leaf.get_name());
    }
}
