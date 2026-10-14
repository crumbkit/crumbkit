use std::io::Cursor;
use std::time::Duration;

use clap::Parser;
use serde_json::{Value, json};

use super::{SelfFigures, figures_from};
use crate::testing::{line, now, tool};
use crumbkit::Rows;
use crumbkit::help::Guide;

/// A log of a help page of the root, two runs of `books list` and one of
/// `books show`, in the last hour.
fn log() -> String {
    [
        line(400, "", &json!({"help": "short"})),
        line(300, "books show", &json!({"out_bytes": 90})),
        line(
            200,
            "books list",
            &json!({"flags": ["--all"], "out_bytes": 500}),
        ),
        line(
            100,
            "books list",
            &json!({"exit": 1, "code": "shelf::books::gone"}),
        ),
    ]
    .concat()
}

/// Returns the JSON of each row of the figures of `log` over the last day,
/// with at most `limit` rows.
fn rows(log: &str, limit: Option<usize>) -> (Vec<Value>, Option<usize>) {
    let listing = figures_from(
        &tool(),
        &Guide::new(),
        Some(Cursor::new(log)),
        Duration::from_secs(86_400),
        now(),
        limit,
    );
    let rows = listing
        .rows()
        .iter()
        .map(|row| serde_json::to_value(row).unwrap())
        .collect();
    (rows, Rows::total(&listing))
}

#[test]
fn each_command_in_the_log_has_a_row_of_its_figures_in_the_order_of_its_path() {
    let (rows, total) = rows(&log(), None);

    assert_eq!(total, Some(3));
    assert_eq!(
        rows,
        [
            json!({"command": "shelf", "runs": 0, "help_pages": 1, "help_before": 0,
                "refused": 0, "failed": 0, "p95_ms": null, "p95_out_bytes": null,
                "typed_full": 0, "typed_all": 0, "typed_long": 0, "exits": []}),
            json!({"command": "books list", "runs": 2, "help_pages": 0, "help_before": 0,
                "refused": 0, "failed": 1, "p95_ms": 10, "p95_out_bytes": 500,
                "typed_full": 0, "typed_all": 1, "typed_long": 0, "exits": []}),
            json!({"command": "books show", "runs": 1, "help_pages": 0, "help_before": 1,
                "refused": 0, "failed": 0, "p95_ms": 10, "p95_out_bytes": 90,
                "typed_full": 0, "typed_all": 0, "typed_long": 0, "exits": []}),
        ]
    );
}

#[test]
fn a_page_shows_the_first_rows_and_counts_every_command_in_its_total() {
    let (rows, total) = rows(&log(), Some(1));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["command"], "shelf");
    assert_eq!(total, Some(3));
}

/// A tool whose only arguments are `self figures list`'s.
#[derive(Parser)]
struct Tool {
    #[command(flatten)]
    args: SelfFigures,
}

#[test]
fn self_figures_list_takes_the_flags_of_self_usage_and_a_page() {
    let tool =
        Tool::try_parse_from(["tool", "--since", "1d", "--log", "copy.jsonl", "--all"]).unwrap();

    assert_eq!(tool.args.usage.since, Duration::from_secs(86_400));
    assert_eq!(tool.args.page.limit(), None);
}

#[test]
fn the_checked_in_schema_matches_the_usage_figures_document_schema() {
    let generated = crumbkit::schemas::document_schema(super::SELF_FIGURES_LIST);
    let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/self-figures-list-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/self-figures-list-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
