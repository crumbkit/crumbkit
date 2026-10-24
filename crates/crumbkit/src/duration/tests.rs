//! Tests for durations: the compact text form, the ISO 8601 JSON form, the
//! schema and the text mode.

use std::time::Duration as Std;

use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use serde_json::json;

use super::Duration;
use crate::shown::TextCells;

/// Returns the JSON form of `duration` as a string, without its quotes.
fn iso(duration: Std) -> String {
    serde_json::to_value(Duration(duration))
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn a_zero_duration_is_pt0s() {
    assert_eq!(iso(Std::ZERO), "PT0S");
}

#[test]
fn a_duration_of_whole_seconds_writes_only_seconds() {
    assert_eq!(iso(Std::from_secs(1)), "PT1S");
    assert_eq!(iso(Std::from_secs(59)), "PT59S");
}

#[test]
fn a_duration_under_a_second_writes_its_fraction_without_trailing_zeros() {
    assert_eq!(iso(Std::from_millis(300)), "PT0.3S");
    assert_eq!(iso(Std::from_millis(250)), "PT0.25S");
    assert_eq!(iso(Std::from_nanos(1)), "PT0.000000001S");
}

#[test]
fn a_fraction_after_whole_seconds_keeps_its_leading_zeros() {
    assert_eq!(iso(Std::new(1, 1)), "PT1.000000001S");
    assert_eq!(iso(Std::from_millis(1_050)), "PT1.05S");
}

#[test]
fn a_duration_of_minutes_and_seconds_writes_both() {
    assert_eq!(iso(Std::from_secs(4 * 60 + 10)), "PT4M10S");
}

#[test]
fn a_part_that_is_zero_is_left_out() {
    assert_eq!(iso(Std::from_secs(60)), "PT1M");
    assert_eq!(iso(Std::from_secs(3_600)), "PT1H");
    assert_eq!(iso(Std::from_secs(3_605)), "PT1H5S");
    assert_eq!(iso(Std::from_secs(3_660)), "PT1H1M");
}

#[test]
fn a_fraction_after_hours_writes_zero_seconds_with_the_fraction() {
    assert_eq!(iso(Std::new(3_600, 500_000_000)), "PT1H0.5S");
}

#[test]
fn days_write_as_hours() {
    assert_eq!(iso(Std::from_secs(3 * 24 * 3_600)), "PT72H");
    assert_eq!(iso(Std::from_secs(29 * 3_600 + 20 * 60)), "PT29H20M");
}

#[test]
fn the_largest_duration_writes_every_hour_and_nanosecond() {
    assert_eq!(iso(Std::MAX), "PT5124095576030431H15.999999999S");
}

#[test]
fn the_text_form_is_the_compact_form_of_an_age() {
    let text = |duration: Std| Duration(duration).to_string();

    assert_eq!(text(Std::from_millis(300)), "0s");
    assert_eq!(text(Std::from_secs(59)), "59s");
    assert_eq!(text(Std::from_secs(4 * 60 + 10)), "4m");
    assert_eq!(text(Std::from_secs(3 * 3_600)), "3h");
    assert_eq!(text(Std::from_secs(3 * 24 * 3_600)), "3d");
    assert_eq!(text(Std::from_secs(13 * 24 * 3_600)), "1w");
}

#[test]
fn a_duration_serializes_as_its_compact_form_while_the_text_mode_is_set() {
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(Duration(Std::from_secs(4 * 60 + 10))).unwrap()
    };

    assert_eq!(value, json!("4m"));
}

#[test]
fn a_duration_converts_from_a_standard_duration() {
    let duration: Duration = Std::from_secs(5).into();

    assert_eq!(duration, Duration(Std::from_secs(5)));
}

#[test]
fn durations_order_by_their_length() {
    assert!(Duration(Std::from_secs(1)) < Duration(Std::from_secs(2)));
}

#[test]
fn the_schema_of_a_duration_is_a_string_in_the_duration_format() {
    let schema = schema_for!(Duration).to_value();

    assert_eq!(schema.get("type"), Some(&json!("string")));
    assert_eq!(schema.get("format"), Some(&json!("duration")));
}

/// A row with a length of time.
#[derive(Serialize, JsonSchema)]
struct Call {
    /// How long the call took.
    took: Duration,
    /// How long the call waited, if it did.
    waited: Option<Duration>,
}

#[test]
fn a_rows_schema_holds_a_duration_field_inline() {
    let schema = schema_for!(Call).to_value();

    assert_eq!(
        schema.pointer("/properties/took/format"),
        Some(&json!("duration"))
    );
    assert_eq!(
        schema.pointer("/properties/took/description"),
        Some(&json!("How long the call took."))
    );
    assert_eq!(
        schema.pointer("/properties/waited/type"),
        Some(&json!(["string", "null"]))
    );
    assert_eq!(schema.get("$defs"), None, "{schema}");
}
