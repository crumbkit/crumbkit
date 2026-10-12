//! Tests for the weights.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{WEIGH, Weight, join};
use crate::tool::Used;

/// Returns the weight of the command at `path` with 500 bytes of `-h`,
/// 1,000 bytes of `--help` and no output.
fn weight(path: &str) -> Weight {
    Weight::new(path.to_owned(), 500, 1_000, None)
}

/// Returns the figures of `command` with `runs` runs, `help_before` help
/// pages before them and a p95 output of `p95`.
fn used(command: &str, runs: u64, help_before: u64, p95: Option<u64>) -> Used {
    Used {
        command: command.to_owned(),
        runs,
        help_before,
        p95_out_bytes: p95,
    }
}

/// Returns the runs, help pages before and p95 output of each of `rows`.
fn figures(rows: &[Weight]) -> Vec<(Option<u64>, Option<u64>, Option<u64>)> {
    rows.iter()
        .map(|row| (row.runs, row.help_before, row.p95_output))
        .collect()
}

#[test]
fn usage_figures_join_each_command_by_its_path_below_the_tool_s_name() {
    let mut rows = vec![
        weight("shelf"),
        weight("shelf books"),
        weight("shelf books list"),
        weight("shelf books show"),
    ];
    let used = [
        used("shelf", 0, 0, None),
        used("books list", 3, 0, Some(512)),
        used("books show", 5, 3, Some(86)),
    ];

    join(&mut rows, &used);

    assert_eq!(
        figures(&rows),
        [
            (Some(0), Some(0), None),
            (Some(0), Some(0), None),
            (Some(3), Some(0), Some(512)),
            (Some(5), Some(3), Some(86)),
        ],
        "a command that the log does not name has no runs"
    );
}

#[test]
fn without_usage_the_figures_are_null() {
    let row = weight("shelf books list");

    assert_eq!(figures(&[row]), [(None, None, None)]);
}

/// The directory of shelf's help golden files.
const SHELF_HELP: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/shelf/tests/golden/help"
);

/// The golden file of weigh's JSON output for shelf.
const SHELF_WEIGHTS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/golden/examples/cargo-crumbkit-weigh-bin-shelf-json.txt"
);

/// weigh reads `-h` pages from the built binary, and crumbkit-test's path
/// weight check renders them in the test process. Both sides then sum them
/// with the same `path_totals`, so equal pages give equal path totals.
#[test]
fn weigh_s_short_help_of_shelf_equals_the_size_of_each_of_shelf_s_help_golden_files() {
    let weights: Value = serde_json::from_str(&fs::read_to_string(SHELF_WEIGHTS).unwrap()).unwrap();
    let rows = weights["data"].as_array().unwrap();
    assert_ne!(rows.len(), 0);
    for row in rows {
        let path = row["path"].as_str().unwrap();
        let golden = Path::new(SHELF_HELP).join(format!("{}-h.txt", path.replace(' ', "-")));
        let size = fs::metadata(&golden).unwrap().len();

        assert_eq!(row["short_help"].as_u64(), Some(size), "{path}");
    }
}

#[test]
fn the_checked_in_schema_matches_the_weigh_document_schema() {
    let generated =
        serde_json::to_string_pretty(&crumbkit::schemas::document_schema(WEIGH)).unwrap() + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/weigh-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/weigh-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}

#[cfg(windows)]
#[test]
fn on_windows_an_absolute_xdg_state_home_holds_the_log_and_otherwise_local_app_data_does() {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::windows_state_home;

    let local = || Some(PathBuf::from(r"C:\Users\me\AppData\Local"));

    assert_eq!(
        windows_state_home(Some(OsString::from(r"C:\state")), local),
        Some(PathBuf::from(r"C:\state"))
    );
    assert_eq!(
        windows_state_home(Some(OsString::from("state")), local),
        local()
    );
    assert_eq!(windows_state_home(None, local), local());
}
