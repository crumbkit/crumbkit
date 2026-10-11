//! Tests for the weights.

use super::{Weight, add_path_totals};

/// Returns the weight of the command at `path` with `short_help` bytes of
/// `-h`, 1,000 bytes of `--help` and `output` bytes of output.
fn weight(path: &str, short_help: u64, output: Option<u64>) -> Weight {
    Weight::new(path.to_owned(), short_help, 1_000, output)
}

/// Returns the path total of each of `rows`.
fn totals(rows: &[Weight]) -> Vec<u64> {
    rows.iter().map(|row| row.path_total).collect()
}

#[test]
fn a_path_total_adds_every_short_page_from_the_root_down_and_the_command_s_output() {
    let mut rows = vec![
        weight("shelf", 500, None),
        weight("shelf books", 300, None),
        weight("shelf books list", 200, Some(150)),
        weight("shelf books show", 250, Some(90)),
        weight("shelf commands", 100, None),
        weight("shelf commands list", 400, Some(2_000)),
    ];

    add_path_totals(&mut rows);

    assert_eq!(
        totals(&rows),
        [500, 800, 1_150, 1_140, 600, 3_000],
        "the long pages never count"
    );
}

#[test]
fn a_command_without_output_weight_totals_its_short_pages_alone() {
    let mut rows = vec![
        weight("shelf", 500, None),
        weight("shelf version", 80, None),
    ];

    add_path_totals(&mut rows);

    assert_eq!(totals(&rows), [500, 580]);
}

#[test]
fn a_root_without_subcommands_totals_its_own_page_and_output() {
    let mut rows = vec![weight("greet", 700, Some(12))];

    add_path_totals(&mut rows);

    assert_eq!(totals(&rows), [712]);
}

#[test]
fn the_checked_in_schema_matches_the_weigh_document_schema() {
    let generated = serde_json::to_string_pretty(&crumbkit::schemas::document_schema(super::WEIGH))
        .unwrap()
        + "\n";
    let checked_in = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/weigh-v1.json"
    ));

    assert_eq!(
        checked_in, generated,
        "schemas/weigh-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
