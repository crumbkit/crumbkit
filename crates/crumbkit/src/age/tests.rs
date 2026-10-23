//! Tests for ages: the compact text form, the JSON form, the schema and the
//! text mode. crumbkit's tests check how `finish_to` and a stream print
//! them.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use serde_json::{Value, json};

use super::{Age, TextCells};

/// The time 2026-10-07T12:01:07.120Z.
fn at() -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(1_791_374_467_120)
}

/// Returns the compact text of an age of `seconds`.
fn text(seconds: u64) -> String {
    Age::since(at(), at() + Duration::from_secs(seconds)).to_string()
}

#[test]
fn an_age_under_a_minute_prints_in_seconds() {
    assert_eq!(text(0), "0s");
    assert_eq!(text(59), "59s");
}

#[test]
fn an_age_from_a_minute_to_under_an_hour_prints_in_minutes() {
    assert_eq!(text(60), "1m");
    assert_eq!(text(119), "1m");
    assert_eq!(text(3_599), "59m");
}

#[test]
fn an_age_from_an_hour_to_under_a_day_prints_in_hours() {
    assert_eq!(text(3_600), "1h");
    assert_eq!(text(3 * 3_600 + 59 * 60), "3h");
    assert_eq!(text(86_399), "23h");
}

#[test]
fn an_age_from_a_day_to_under_a_week_prints_in_days() {
    assert_eq!(text(86_400), "1d");
    assert_eq!(text(2 * 86_400 + 86_399), "2d");
}

#[test]
fn an_age_of_six_days_and_23_hours_prints_six_days() {
    assert_eq!(text(6 * 86_400 + 23 * 3_600), "6d");
}

#[test]
fn an_age_of_exactly_seven_days_prints_one_week() {
    assert_eq!(text(7 * 86_400), "1w");
}

#[test]
fn an_age_of_13_days_prints_one_week() {
    assert_eq!(text(13 * 86_400), "1w");
}

#[test]
fn an_age_of_14_days_prints_two_weeks() {
    assert_eq!(text(14 * 86_400), "2w");
}

#[test]
fn an_age_of_400_days_prints_57_weeks() {
    assert_eq!(text(400 * 86_400), "57w");
}

/// Returns the compact text of a measured duration of `seconds`.
fn compact(seconds: u64) -> String {
    Age::compact(Duration::from_secs(seconds)).to_string()
}

#[test]
fn a_measured_duration_under_a_minute_prints_in_seconds() {
    assert_eq!(compact(0), "0s");
    assert_eq!(compact(3), "3s");
    assert_eq!(
        Age::compact(Duration::from_millis(59_999)).to_string(),
        "59s"
    );
}

#[test]
fn a_measured_duration_of_59_minutes_and_59_seconds_prints_59_minutes() {
    assert_eq!(compact(59 * 60 + 59), "59m");
}

#[test]
fn a_measured_duration_of_exactly_a_day_prints_one_day() {
    assert_eq!(compact(86_400), "1d");
}

#[test]
fn a_measured_duration_of_13_days_prints_one_week() {
    assert_eq!(compact(13 * 86_400), "1w");
}

#[test]
fn compact_and_since_print_the_same_text_for_the_same_span() {
    let spans = [
        0,
        1,
        59,
        60,
        61,
        3_599,
        3_600,
        3_601,
        86_399,
        86_400,
        86_401,
        7 * 86_400 - 1,
        7 * 86_400,
        13 * 86_400,
        14 * 86_400,
        400 * 86_400,
    ];

    for seconds in spans {
        assert_eq!(compact(seconds), text(seconds), "{seconds} seconds");
    }
}

#[test]
fn an_age_ignores_the_fraction_of_a_second() {
    let age = Age::since(at(), at() + Duration::from_millis(59_999));

    assert_eq!(age.to_string(), "59s");
}

#[test]
fn an_age_holds_its_time_and_the_duration_from_it_to_now() {
    let age = Age::since(at(), at() + Duration::from_secs(90));

    assert_eq!(age.at(), at());
    assert_eq!(age.elapsed(), Duration::from_secs(90));
}

#[test]
fn a_time_later_than_now_has_an_age_of_zero() {
    let age = Age::since(at() + Duration::from_secs(5), at());

    assert_eq!(age.at(), at() + Duration::from_secs(5));
    assert_eq!(age.elapsed(), Duration::ZERO);
    assert_eq!(age.to_string(), "0s");
}

#[test]
fn an_age_serializes_as_its_time_in_rfc_3339_with_milliseconds() {
    let age = Age::since(at(), at() + Duration::from_secs(3 * 3_600));

    assert_eq!(
        serde_json::to_value(age).unwrap(),
        json!("2026-10-07T12:01:07.120Z")
    );
    assert_eq!(
        serde_json::to_string(&age).unwrap(),
        r#""2026-10-07T12:01:07.120Z""#
    );
}

#[test]
fn an_age_serializes_as_its_compact_age_while_the_text_mode_is_set() {
    let age = Age::since(at(), at() + Duration::from_secs(3 * 3_600));

    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(age).unwrap()
    };

    assert_eq!(value, json!("3h"));
}

#[test]
fn the_text_mode_ends_when_its_guard_is_dropped() {
    let age = Age::since(at(), at() + Duration::from_secs(3 * 3_600));

    drop(TextCells::enter());

    assert_eq!(
        serde_json::to_value(age).unwrap(),
        json!("2026-10-07T12:01:07.120Z")
    );
}

#[test]
fn an_inner_guard_restores_the_text_mode_that_it_found() {
    let age = Age::since(at(), at() + Duration::from_secs(60));
    let outer = TextCells::enter();

    drop(TextCells::enter());
    let value = serde_json::to_value(age).unwrap();
    drop(outer);

    assert_eq!(value, json!("1m"));
}

#[test]
fn the_text_mode_of_one_thread_leaves_another_thread_in_rfc_3339() {
    let age = Age::since(at(), at() + Duration::from_secs(60));
    let _cells = TextCells::enter();

    let other = std::thread::spawn(move || serde_json::to_value(age).unwrap())
        .join()
        .unwrap();

    assert_eq!(other, json!("2026-10-07T12:01:07.120Z"));
}

#[test]
fn a_time_before_1970_fails_to_serialize_as_rfc_3339() {
    let before = UNIX_EPOCH - Duration::from_secs(1);
    let age = Age::since(before, at());

    let error = serde_json::to_value(age).unwrap_err();

    assert!(error.to_string().contains("1970"), "{error}");
}

#[test]
fn a_time_after_the_year_9999_fails_to_serialize_as_rfc_3339() {
    let after = UNIX_EPOCH + Duration::from_secs(253_402_300_800);
    let age = Age::since(after, after);

    let error = serde_json::to_value(age).unwrap_err();

    assert!(error.to_string().contains("9999"), "{error}");
}

#[test]
fn a_time_before_1970_still_prints_its_compact_age_in_text() {
    let before = UNIX_EPOCH - Duration::from_secs(1);
    let age = Age::since(before, UNIX_EPOCH + Duration::from_secs(1));

    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(age).unwrap()
    };

    assert_eq!(value, json!("2s"));
}

#[test]
fn the_schema_of_an_age_is_a_string_in_the_date_time_format() {
    let schema = schema_for!(Age).to_value();

    assert_eq!(schema.get("type"), Some(&json!("string")));
    assert_eq!(schema.get("format"), Some(&json!("date-time")));
    assert!(
        schema
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("RFC 3339")),
        "{schema}"
    );
}

/// A row with a time.
#[derive(Serialize, JsonSchema)]
struct Entry {
    /// When the entry was added.
    added: Age,
    /// When the entry was last read, if it was.
    read: Option<Age>,
}

#[test]
fn a_rows_schema_holds_an_age_field_inline_as_a_date_time_string() {
    let schema = schema_for!(Entry).to_value();

    let added = schema.pointer("/properties/added");
    assert_eq!(
        added.and_then(|added| added.get("format")),
        Some(&json!("date-time"))
    );
    assert_eq!(
        added.and_then(|added| added.get("description")),
        Some(&json!("When the entry was added."))
    );
    let read = schema.pointer("/properties/read");
    assert_eq!(
        read.and_then(|read| read.get("type")),
        Some(&json!(["string", "null"]))
    );
    assert_eq!(schema.get("$defs"), None, "{schema}");
}
