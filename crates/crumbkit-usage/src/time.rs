//! The times of the usage log: RFC 3339 in UTC, and the start of a
//! `--since` period.

use std::time::{SystemTime, UNIX_EPOCH};

use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

/// Returns `time` as a jiff timestamp, with a time after jiff's last
/// timestamp, 9999-12-30T22:00:00.999999999Z, read as that one and a time before 1970
/// read as the epoch, so that a wrong clock still has a form.
fn clamped(time: SystemTime) -> Timestamp {
    Timestamp::try_from(time.max(UNIX_EPOCH)).unwrap_or(Timestamp::MAX)
}

/// Returns `time` in RFC 3339 with milliseconds, such as
/// `2026-10-07T12:01:07.120Z`.
pub(crate) fn millis(time: SystemTime) -> String {
    format!("{:.3}", clamped(time))
}

/// Returns `time` in RFC 3339 with seconds, such as `2026-10-07T12:01:07Z`.
pub(crate) fn seconds(time: SystemTime) -> String {
    format!("{:.0}", clamped(time))
}

/// Returns the UTC day of `time` as `YYYY-MM-DD`.
pub(crate) fn day_of(time: SystemTime) -> String {
    clamped(time)
        .to_zoned_datetime(TimeZone::UTC)
        .date()
        .to_string()
}

/// Reads an RFC 3339 time, such as `2026-10-07T12:01:07.120Z`.
pub(crate) fn parse(text: &str) -> Result<SystemTime, jiff::Error> {
    text.parse::<Timestamp>().and_then(SystemTime::try_from)
}

/// Returns the time `span` before `now` on the UTC calendar, so a month is
/// a calendar month and a day is a calendar day. A start before 1970, or one
/// that jiff cannot count to, is the epoch, because no record is older.
pub(crate) fn before(now: SystemTime, span: Span) -> SystemTime {
    clamped(now)
        .to_zoned_datetime(TimeZone::UTC)
        .checked_sub(span)
        .and_then(|start| SystemTime::try_from(start.timestamp()))
        .map_or(UNIX_EPOCH, |start| start.max(UNIX_EPOCH))
}

#[cfg(test)]
mod tests;
