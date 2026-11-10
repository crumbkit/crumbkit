use std::io::Cursor;
use std::time::Duration;

use clap::Parser;
use serde_json::{Value, json};

use super::{SelfRuns, runs_from};
use crate::self_usage::Window;
use crate::testing::{line, marked_log, now, tree, window, without_flightdeck};
use crumbkit::Tool;

/// The keyed hash of the second session in [`log`].
const OTHER: &str = "fedcba9876543210";

/// A log of two sessions whose runs interleave: the first session's help
/// page of the root and `books list`, and the second session's
/// `books show` between them.
fn log() -> String {
    [
        line(400, "", &json!({"help": "short", "out_bytes": null})),
        line(
            300,
            "books show",
            &json!({"session": OTHER, "exit": 1, "code": "shelf::books::not_found",
                "parent": "b7ad6b7169203331"}),
        ),
        line(
            200,
            "books list",
            &json!({"flags": ["--all"], "out_bytes": 500, "truncated": true}),
        ),
    ]
    .concat()
}

/// Returns the JSON of each row of the runs of `log` over the last day,
/// with at most `limit` rows, the listing's total and its summary.
fn rows(log: &str, limit: Option<usize>) -> (Vec<Value>, Option<usize>, Value) {
    let listing = runs_from(
        &tree(),
        &Tool::new(),
        Some(Cursor::new(log)),
        &window(Duration::from_secs(86_400), now()),
        limit,
    );
    let rows = listing
        .rows()
        .iter()
        .map(|row| serde_json::to_value(row).unwrap())
        .collect();
    let summary = serde_json::to_value(listing.get_summary()).unwrap();
    (rows, listing.total(), summary)
}

#[test]
fn each_run_is_a_row_grouped_by_session_in_the_order_of_each_sessions_first_run() {
    let (rows, total, _) = rows(&log(), None);

    let places: Vec<(&Value, &Value, &Value)> = rows
        .iter()
        .map(|row| (&row["session"], &row["step"], &row["command"]))
        .collect();
    assert_eq!(total, Some(3));
    assert_eq!(
        places,
        [
            (&json!(1), &json!(1), &json!("shelf")),
            (&json!(1), &json!(2), &json!("books list")),
            (&json!(2), &json!(1), &json!("books show")),
        ]
    );
}

#[test]
fn a_row_holds_the_shape_of_its_run_its_end_to_the_second_and_its_trace_ids() {
    let (rows, _, _) = rows(&log(), None);

    assert_eq!(
        rows[1],
        json!({"session": 1, "step": 2, "at": "2026-10-11T11:56:40Z", "caller": "person",
            "command": "books list", "flags": ["--all"], "help": false, "exit": 0,
            "code": null, "ms": 10, "out_bytes": 500, "truncated": true,
            "trace": "4bf92f3577b34da6a3ce929d0e0e4736", "run": "00f067aa0ba902b7",
            "parent": null})
    );
    assert_eq!(rows[2]["code"], "shelf::books::not_found");
    assert_eq!(rows[2]["parent"], "b7ad6b7169203331");
    assert_eq!(rows[0]["help"], true);
    assert_eq!(rows[0]["out_bytes"], Value::Null);
}

#[test]
fn a_run_whose_trace_id_is_lowercase_hex_of_32_digits_is_listed() {
    let log = line(100, "books show", &json!({}));

    let (rows, _, summary) = rows(&log, None);

    assert_eq!(rows.len(), 1);
    assert_eq!(summary["skipped"], 0);
}

#[test]
fn a_run_whose_trace_id_is_not_lowercase_hex_of_32_digits_is_skipped_and_counted() {
    let log = line(
        100,
        "books show",
        &json!({"trace": "<script>alert(1)</script>"}),
    );

    let (rows, _, summary) = rows(&log, None);

    assert_eq!(rows, Vec::<Value>::new());
    assert_eq!(summary["skipped"], 1);
}

#[test]
fn the_summary_is_the_summary_of_self_usage() {
    let (_, _, summary) = rows(&log(), None);

    assert_eq!(summary["runs"], 3);
    assert_eq!(summary["sessions"], 2);
}

#[test]
fn a_page_shows_the_first_rows_and_counts_every_run_in_its_total() {
    let (rows, total, _) = rows(&log(), Some(1));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["command"], "shelf");
    assert_eq!(total, Some(3));
}

/// A tool whose only arguments are `self runs list`'s.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    args: SelfRuns,
}

#[test]
fn self_runs_list_takes_the_flags_of_self_usage_and_a_page() {
    let tool =
        Cli::try_parse_from(["tool", "--since", "1d", "--log", "copy.jsonl", "--all"]).unwrap();

    assert_eq!(tool.args.usage.since, Duration::from_secs(86_400));
    assert_eq!(tool.args.page.limit(), None);
}

#[test]
fn the_checked_in_schema_matches_the_runs_document_schema() {
    let generated = crumbkit::schemas::document_schema(super::SELF_RUNS_LIST);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/self-runs-list-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/self-runs-list-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}

/// Returns the command of each row of the runs of [`marked_log`] over
/// `window`.
fn commands(window: &Window<'_>) -> Vec<String> {
    let listing = runs_from(
        &tree(),
        &Tool::new(),
        Some(Cursor::new(marked_log())),
        window,
        None,
    );
    listing
        .rows()
        .iter()
        .map(|row| {
            serde_json::to_value(row).unwrap()["command"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn the_runs_leave_out_those_of_an_excluded_program_and_keep_those_of_another() {
    assert_eq!(
        commands(&without_flightdeck()),
        ["books list", "books show"]
    );
    assert_eq!(
        commands(&window(Duration::from_secs(86_400), now())),
        ["books list", "books list", "books show"]
    );
}
