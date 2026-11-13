use std::io::Cursor;
use std::ops::ControlFlow;

use clap::Parser;
use serde_json::{Value, json};

use super::{RunLine, SelfRunsExport, each_line, span_id};
use crate::testing::line;
use crumbkit::Row as _;

/// Returns the line of a run of `command` with the span id `run` that ended
/// `secs` seconds before the tests' time, in session `s1`.
fn run_line(secs: u64, command: &str, run: &str) -> String {
    line(secs, command, &json!({"run": run}))
}

/// A log of three runs, `a`, `b` and `c`, in that order, 300, 200 and 100
/// seconds before the tests' time.
fn log() -> String {
    [
        run_line(300, "books list", "000000000000000a"),
        run_line(200, "books show", "000000000000000b"),
        run_line(100, "", "000000000000000c"),
    ]
    .concat()
}

/// Returns the lines that an export of `log` prints after `after`.
fn export(log: &str, after: Option<&str>) -> Vec<RunLine> {
    let mut lines = Vec::new();
    each_line("shelf", Cursor::new(log), after, |line| {
        lines.push(line);
        ControlFlow::Continue(())
    });
    lines
}

/// Returns the JSON of each line that an export of `log` prints after
/// `after`.
fn json(log: &str, after: Option<&str>) -> Vec<Value> {
    export(log, after)
        .iter()
        .map(|line| serde_json::to_value(line).unwrap())
        .collect()
}

/// Returns the span id of each record that an export of `log` prints after
/// `after`, and whether each line follows a gap.
fn runs(log: &str, after: Option<&str>) -> Vec<(String, bool)> {
    json(log, after)
        .into_iter()
        .map(|line| {
            (
                line["record"]["run"].as_str().unwrap().to_owned(),
                line["gap"].as_bool().unwrap(),
            )
        })
        .collect()
}

#[test]
fn an_export_without_after_prints_every_record_oldest_first() {
    let runs = runs(&log(), None);

    assert_eq!(
        runs,
        [
            ("000000000000000a".to_owned(), false),
            ("000000000000000b".to_owned(), false),
            ("000000000000000c".to_owned(), false),
        ]
    );
}

#[test]
fn each_line_holds_the_record_as_the_log_stores_it_with_its_hashes() {
    let log = log();
    let first = log.lines().next().unwrap();

    let lines = export(&log, None);

    let stored: Value = serde_json::from_str(first).unwrap();
    assert_eq!(serde_json::to_value(&lines[0].record).unwrap(), stored);
    let record = &json(&log, None)[0]["record"];
    assert_eq!(record["session"], "0123456789abcdef");
    assert_eq!(record["args"], format!("{:016x}", 300));
}

#[test]
fn an_export_after_a_run_prints_only_the_records_that_follow_it() {
    let runs = runs(&log(), Some("000000000000000a"));

    assert_eq!(
        runs,
        [
            ("000000000000000b".to_owned(), false),
            ("000000000000000c".to_owned(), false),
        ]
    );
}

#[test]
fn an_export_after_the_last_run_prints_nothing() {
    let runs = runs(&log(), Some("000000000000000c"));

    assert_eq!(runs, []);
}

#[test]
fn an_export_after_a_run_that_was_trimmed_prints_every_record_and_marks_the_first_after_a_gap() {
    let runs = runs(&log(), Some("000000000000000f"));

    assert_eq!(
        runs,
        [
            ("000000000000000a".to_owned(), true),
            ("000000000000000b".to_owned(), false),
            ("000000000000000c".to_owned(), false),
        ]
    );
}

#[test]
fn an_export_after_a_trimmed_run_prints_a_record_that_ended_earlier_but_was_appended_later() {
    // A run waits for the lock after it ends, so the log's order is not
    // the order of the end times.
    let log = [
        run_line(100, "books list", "000000000000000a"),
        run_line(200, "books show", "000000000000000b"),
    ]
    .concat();

    let runs = runs(&log, Some("000000000000000f"));

    assert_eq!(runs.len(), 2);
}

#[test]
fn a_last_line_without_a_newline_is_a_write_in_progress_and_does_not_print() {
    let log = log();
    let log = log.strip_suffix('\n').unwrap();

    let runs = runs(log, None);

    assert_eq!(runs.len(), 2);
}

#[test]
fn a_record_whose_span_id_is_not_16_lowercase_hex_digits_is_skipped() {
    let log = [run_line(400, "books list", "NOT-A-SPAN-ID"), log()].concat();

    let runs = runs(&log, None);

    assert_eq!(runs.len(), 3);
}

#[test]
fn a_line_that_is_not_a_usage_record_is_skipped() {
    let log = [
        "not json\n".to_owned(),
        "{\"schema\":\"shelf/books-list/v1\"}\n".to_owned(),
        log(),
    ]
    .concat();

    let runs = runs(&log, None);

    assert_eq!(runs.len(), 3);
}

#[test]
fn an_export_stops_reading_when_the_stream_stops() {
    let mut lines = 0;
    each_line("shelf", Cursor::new(log()), None, |_| {
        lines += 1;
        ControlFlow::Break(())
    });

    assert_eq!(lines, 1);
}

#[test]
fn a_line_prints_its_run_command_and_exit_as_text() {
    let lines = export(&log(), Some("000000000000000f"));

    let text = |run: &RunLine| RunLine::LINE.and_then(|line| line.text(run)).unwrap();
    assert_eq!(
        text(&lines[0]),
        "000000000000000a  books list  0  after a gap"
    );
    assert_eq!(text(&lines[2]), "000000000000000c  shelf  0");
}

#[test]
fn a_run_is_its_span_id_of_16_lowercase_hex_digits() {
    assert_eq!(
        span_id("00f067aa0ba902b7").as_deref(),
        Ok("00f067aa0ba902b7")
    );
    for text in [
        "00F067AA0BA902B7",
        "00f067aa0ba902",
        "00f067aa0ba902b7@2026-10-11T11:56:40.120Z",
    ] {
        assert!(span_id(text).is_err(), "{text}");
    }
}

/// A tool whose only arguments are `self runs export`'s.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: SelfRunsExport,
}

#[test]
fn self_runs_export_takes_a_run_after_and_a_log() {
    let tool = Cli::try_parse_from(["tool", "--after", "00f067aa0ba902b7", "--log", "copy.jsonl"])
        .unwrap();

    assert_eq!(tool.args.after.as_deref(), Some("00f067aa0ba902b7"));
}

#[test]
fn the_checked_in_schema_matches_the_export_document_schema() {
    // A stream's line has the schema of a document of one row, which
    // `schemas show` serves through the leaf.
    let line: crumbkit::Document<RunLine> =
        crumbkit::Document::new("crumbkit", "self-runs-export", 1, super::DESCRIPTION);
    assert_eq!(line.to_string(), super::SELF_RUNS_EXPORT.to_string());
    let generated = crumbkit::schemas::document_schema(line);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/self-runs-export-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/self-runs-export-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
