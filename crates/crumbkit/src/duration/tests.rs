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
    serde_json::to_value(Duration::new(duration))
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
    assert_eq!(iso(Std::from_secs(3_660)), "PT1H1M");
}

#[test]
fn seconds_after_hours_write_zero_minutes_between_them() {
    assert_eq!(iso(Std::from_secs(3_605)), "PT1H0M5S");
    assert_eq!(iso(Std::new(3_600, 500_000_000)), "PT1H0M0.5S");
}

#[test]
fn days_write_as_hours() {
    assert_eq!(iso(Std::from_secs(3 * 24 * 3_600)), "PT72H");
    assert_eq!(iso(Std::from_secs(29 * 3_600 + 20 * 60)), "PT29H20M");
}

#[test]
fn the_largest_duration_writes_every_hour_and_nanosecond() {
    assert_eq!(iso(Std::MAX), "PT5124095576030431H0M15.999999999S");
}

/// Returns the text form of `duration`.
fn text(duration: Std) -> String {
    Duration::new(duration).to_string()
}

#[test]
fn a_zero_duration_reads_0s() {
    assert_eq!(text(Std::ZERO), "0s");
}

#[test]
fn a_duration_under_a_second_reads_in_its_largest_whole_sub_second_unit() {
    assert_eq!(text(Std::from_millis(300)), "300ms");
    assert_eq!(text(Std::from_micros(999)), "999\u{b5}s");
    assert_eq!(text(Std::from_nanos(999_999_999)), "999ms");
    assert_eq!(text(Std::from_nanos(500)), "500ns");
}

#[test]
fn a_duration_under_a_minute_reads_in_seconds_with_one_decimal_place() {
    assert_eq!(text(Std::from_secs(1)), "1s");
    assert_eq!(text(Std::from_millis(1_050)), "1s");
    assert_eq!(text(Std::from_millis(1_900)), "1.9s");
    assert_eq!(text(Std::from_secs(45)), "45s");
    assert_eq!(text(Std::from_millis(59_900)), "59.9s");
}

#[test]
fn a_duration_just_under_a_minute_rounds_down_to_59_9s() {
    assert_eq!(text(Std::from_millis(59_990)), "59.9s");
    assert_eq!(text(Std::from_nanos(59_999_999_999)), "59.9s");
}

#[test]
fn a_duration_of_a_minute_or_more_drops_its_fraction_of_a_second() {
    assert_eq!(text(Std::from_millis(60_900)), "1m");
    assert_eq!(text(Std::from_millis(70_900)), "1m 10s");
}

#[test]
fn a_duration_reads_its_largest_unit_and_the_next_one_when_that_is_not_zero() {
    assert_eq!(text(Std::from_secs(4 * 60 + 10)), "4m 10s");
    assert_eq!(text(Std::from_secs(2 * 60)), "2m");
    assert_eq!(text(Std::from_secs(3_600)), "1h");
    assert_eq!(text(Std::from_secs(3_600 + 5 * 60)), "1h 5m");
    assert_eq!(text(Std::from_secs(3 * 86_400 + 2 * 3_600)), "3d 2h");
    assert_eq!(text(Std::from_secs(8 * 86_400)), "1w 1d");
}

#[test]
fn a_smaller_unit_after_a_zero_next_unit_is_left_out() {
    assert_eq!(text(Std::from_secs(3_600 + 5)), "1h");
    assert_eq!(text(Std::from_secs(86_400 + 59 * 60)), "1d");
}

#[test]
fn a_duration_serializes_as_its_compact_form_while_the_text_mode_is_set() {
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(Duration::new(Std::from_secs(4 * 60 + 10))).unwrap()
    };

    assert_eq!(value, json!("4m 10s"));
}

#[test]
fn a_duration_converts_from_a_standard_duration() {
    let duration: Duration = Std::from_secs(5).into();

    assert_eq!(duration, Duration::new(Std::from_secs(5)));
}

#[test]
fn a_duration_returns_the_standard_duration_it_was_made_from() {
    let standard = Std::from_millis(1_500);

    assert_eq!(Duration::new(standard).get(), standard);
}

#[test]
fn a_duration_made_with_new_is_usable_in_a_constant() {
    const TIMEOUT: Duration = Duration::new(Std::from_secs(30));

    assert_eq!(TIMEOUT.get(), Std::from_secs(30));
}

#[test]
fn durations_order_by_their_length() {
    assert!(Duration::new(Std::from_secs(1)) < Duration::new(Std::from_secs(2)));
}

#[test]
fn the_schema_of_a_duration_is_a_string_with_a_pattern_and_no_format() {
    let schema = schema_for!(Duration).to_value();

    assert_eq!(schema.get("type"), Some(&json!("string")));
    assert_eq!(schema.get("pattern"), Some(&json!(super::PATTERN)));
    assert_eq!(schema.get("format"), None);
}

/// Returns the pattern of the schema as a regular expression.
fn pattern() -> Option<regex_lite::Regex> {
    regex_lite::Regex::new(super::PATTERN).ok()
}

#[test]
fn the_pattern_accepts_every_form_that_a_duration_writes() {
    let pattern = pattern().unwrap();
    let durations = [
        Std::ZERO,
        Std::from_nanos(1),
        Std::from_millis(300),
        Std::from_millis(250),
        Std::from_secs(1),
        Std::new(1, 1),
        Std::from_millis(1_050),
        Std::from_secs(59),
        Std::new(59, 999_999_999),
        Std::from_secs(60),
        Std::new(60, 500_000_000),
        Std::from_secs(4 * 60 + 10),
        Std::from_secs(59 * 60 + 59),
        Std::from_secs(3_600),
        Std::from_secs(3_605),
        Std::new(3_600, 500_000_000),
        Std::from_secs(3_660),
        Std::from_secs(3_661),
        Std::from_secs(29 * 3_600 + 20 * 60),
        Std::from_secs(3 * 24 * 3_600),
        Std::MAX,
    ];
    for duration in durations {
        let written = iso(duration);
        assert!(pattern.is_match(&written), "{written}");
    }
}

#[test]
fn the_pattern_refuses_forms_that_a_duration_never_writes() {
    let pattern = pattern().unwrap();
    for refused in [
        "PT",
        "P3D",
        "PT1H5S",
        "PT1H0M",
        "PT1M0S",
        "PT60S",
        "PT60M",
        "PT01M",
        "PT0H5M",
        "PT1.50S",
        "PT1.0S",
        "PT0.0000000001S",
        "PT4M10S ",
    ] {
        assert!(!pattern.is_match(refused), "{refused}");
    }
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
        schema.pointer("/properties/took/pattern"),
        Some(&json!(super::PATTERN))
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
