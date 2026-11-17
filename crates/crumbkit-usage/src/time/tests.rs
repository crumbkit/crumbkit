use std::time::{Duration, UNIX_EPOCH};

use super::*;

#[test]
fn the_epoch_writes_with_milliseconds_and_with_seconds() {
    assert_eq!(millis(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    assert_eq!(seconds(UNIX_EPOCH), "1970-01-01T00:00:00Z");
}

#[test]
fn a_millisecond_time_writes_its_three_digits_and_the_seconds_form_drops_them() {
    let time = UNIX_EPOCH + Duration::from_millis(1_791_720_067_120);

    assert_eq!(millis(time), "2026-10-11T12:01:07.120Z");
    assert_eq!(seconds(time), "2026-10-11T12:01:07Z");
}

#[test]
fn the_reader_reads_what_the_writer_writes_and_a_zero_offset() {
    let time = UNIX_EPOCH + Duration::from_millis(1_791_720_067_120);

    assert_eq!(parse(&millis(time)).unwrap(), time);
    assert_eq!(parse("2026-10-11T12:01:07.120+00:00").unwrap(), time);
}

#[test]
fn a_time_outside_the_four_digit_years_clamps_to_the_last_form_jiff_has() {
    let far = UNIX_EPOCH + Duration::from_secs(300_000_000_000);

    assert_eq!(seconds(far), "9999-12-30T22:00:00Z");
    assert_eq!(day_of(far), "9999-12-30");
}
