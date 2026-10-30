//! A length of time in a row, which prints as a compact duration in text
//! and as an ISO 8601 duration in JSON.

use std::fmt;

use schemars::{Schema, SchemaGenerator, json_schema};
use serde::Serializer;

use crate::Shown;

#[cfg(test)]
mod tests;

/// The seconds in a minute.
const MINUTE: u64 = 60;

/// The seconds in an hour.
const HOUR: u64 = 60 * MINUTE;

/// The seconds in a day.
const DAY: u64 = 24 * HOUR;

/// The seconds in a week.
const WEEK: u64 = 7 * DAY;

/// The units of the compact forms of [`Age`](crate::Age) and [`Duration`],
/// largest first, each with the seconds it holds and its spelling.
const UNITS: [(u64, char); 5] = [
    (WEEK, 'w'),
    (DAY, 'd'),
    (HOUR, 'h'),
    (MINUTE, 'm'),
    (1, 's'),
];

/// The units under a second of the text form of a [`Duration`], largest
/// first, each with the nanoseconds it holds and its spelling.
const SUB_SECOND_UNITS: [(u32, &str); 3] = [(1_000_000, "ms"), (1_000, "\u{b5}s"), (1, "ns")];

/// The pattern of the JSON form of a [`Duration`], which matches every
/// string that [`Iso8601`] writes and no other: whole hours without leading
/// zeros, minutes and seconds from 0 to 59 without leading zeros, and a
/// fraction of a second of one to nine digits without trailing zeros.
const PATTERN: &str = concat!(
    r"^PT(?:",
    // Hours, then minutes and seconds that are not zero, or minutes alone.
    r"[1-9][0-9]*H(?:[1-5]?[0-9]M",
    r"(?:(?:[1-5][0-9]|[1-9])(?:\.[0-9]{0,8}[1-9])?|0\.[0-9]{0,8}[1-9])S",
    r"|(?:[1-5][0-9]|[1-9])M)?",
    // Minutes that are not zero, then seconds that are not zero.
    r"|(?:[1-5][0-9]|[1-9])M",
    r"(?:(?:(?:[1-5][0-9]|[1-9])(?:\.[0-9]{0,8}[1-9])?|0\.[0-9]{0,8}[1-9])S)?",
    // Seconds alone, which may be zero.
    r"|[1-5]?[0-9](?:\.[0-9]{0,8}[1-9])?S",
    r")$",
);

/// The type of a row field or a failure field that holds a length of time,
/// such as how long a request took. Text shows it in a short form, such as
/// `4m 10s` or `1.9s`, and JSON holds it as an ISO 8601 duration, such as
/// `PT4M10S`.
///
/// The text form uses the unit spellings of an [`Age`](crate::Age) column,
/// `s`, `m`, `h`, `d` and `w`, and depends on the length:
///
/// - Under one second it is the largest of milliseconds, microseconds and
///   nanoseconds with a whole count, such as `300ms`, `999µs` or `500ns`. A
///   zero duration reads `0s`.
/// - From one second to under a minute it is seconds with one decimal
///   place, without a trailing `.0`, such as `1s`, `1.9s` or `59.9s`. The
///   tenths round down, so 59.99 seconds reads `59.9s` and never `60s`.
/// - From one minute up it is the largest unit with a whole count, then the
///   next smaller unit when its count is not zero, such as `2m`, `4m 10s`,
///   `1h`, `1h 5m`, `3d 2h` or `1w 1d`. A unit two places down never
///   prints, so 1 hour and 5 seconds reads `1h`, and each count rounds down.
///
/// The JSON form is exact to the nanosecond. It is `PT`, then the whole
/// hours with `H`, the minutes with `M` and the seconds with `S`, such as
/// `PT4M10S`. It leaves out a part that is zero, except that minutes always
/// come between hours and seconds, so 1 hour and 5 seconds is `PT1H0M5S`.
/// The seconds carry a fraction when the duration has one, with no
/// trailing zeros, such as `PT0.3S` or `PT1.000000001S`. A zero duration is
/// `PT0S`. Hours are the largest unit, so three days are `PT72H`, because
/// ISO 8601 reads a day as a calendar day, which a change of daylight
/// saving time can make 23 or 25 hours long.
///
/// The field's JSON Schema is an inline string with a `pattern` that
/// matches exactly the strings that `Duration` writes. It declares no
/// `format`, because the JSON Schema format `duration` follows the grammar
/// in appendix A of RFC 3339, which has no fraction of a second, so a
/// validator that checks formats would refuse every duration under a
/// second.
///
/// `Duration` implements [`Shown`], so the text table, `--fields` text and
/// a stream's text line print the text form, and every JSON form, including
/// a failure field from [`Field::shown`](crate::Field::shown), holds the
/// ISO 8601 duration.
///
/// ```
/// use crumbkit::Duration;
///
/// let took = Duration::new(std::time::Duration::from_secs(4 * 60 + 10));
///
/// assert_eq!(took.to_string(), "4m 10s");
/// assert_eq!(took.get(), std::time::Duration::from_secs(250));
/// assert_eq!(serde_json::to_string(&took).unwrap(), r#""PT4M10S""#);
/// ```
//
// RUST-10: `Duration` serializes and has no `Deserialize`, because crumbkit
// prints documents and does not read them back. Reading the ISO 8601 form
// needs a parser for the general form, with days, weeks and fractions on
// any part, and the iso8601 crate or a parser of that size replaces this
// once a tool reads such documents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(std::time::Duration);

impl Duration {
    /// Wraps a standard duration, so a row field prints it in the compact
    /// text form and as an ISO 8601 duration in JSON.
    #[must_use]
    pub const fn new(duration: std::time::Duration) -> Self {
        Self(duration)
    }

    /// Returns the standard duration that this value wraps.
    #[must_use]
    pub const fn get(self) -> std::time::Duration {
        self.0
    }
}

impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Self(duration)
    }
}

/// Writes the duration in its largest unit and the next one, such as
/// `4m 10s`, or in its largest unit under a second, such as `300ms`.
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        if seconds == 0 {
            let nanos = self.0.subsec_nanos();
            return match SUB_SECOND_UNITS.iter().find(|(size, _)| nanos >= *size) {
                Some((size, unit)) => write!(f, "{}{unit}", nanos / size),
                None => f.write_str("0s"),
            };
        }
        if seconds < MINUTE {
            // Tenths of a second, rounded down, so 59.99 seconds reads
            // `59.9s` and never `60s`.
            let tenths = self.0.subsec_nanos() / 100_000_000;
            return if tenths == 0 {
                write!(f, "{seconds}s")
            } else {
                write!(f, "{seconds}.{tenths}s")
            };
        }
        let mut units = UNITS.iter().skip_while(|(size, _)| seconds < *size);
        if let Some((size, unit)) = units.next() {
            write!(f, "{}{unit}", seconds / size)?;
            if let Some((next, next_unit)) = units.next() {
                let count = seconds % size / next;
                if count > 0 {
                    write!(f, " {count}{next_unit}")?;
                }
            }
        }
        Ok(())
    }
}

/// A length of time whose `Display` writes its largest whole unit only,
/// such as `3h`, the text form of an [`Age`](crate::Age).
pub(crate) struct Largest(pub(crate) std::time::Duration);

impl fmt::Display for Largest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        let (size, unit) = UNITS
            .iter()
            .find(|(size, _)| seconds >= *size)
            .copied()
            .unwrap_or((1, 's'));
        write!(f, "{}{unit}", seconds / size)
    }
}

/// Writes the JSON form as an ISO 8601 duration in hours, minutes and
/// seconds, and describes it as an inline string with a `pattern` and no
/// `format`.
impl Shown for Duration {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&Iso8601(self.0))
    }

    fn schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A length of time as an ISO 8601 duration with hours as its largest unit and an optional decimal fraction of a second, such as PT4M10S, PT0.3S or PT72H.",
            "type": "string",
            "pattern": PATTERN,
        })
    }
}

crate::shown!(Duration);

/// A duration whose `Display` writes it in ISO 8601, such as `PT4M10S`.
struct Iso8601(std::time::Duration);

impl fmt::Display for Iso8601 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        let nanos = self.0.subsec_nanos();
        let (hours, minutes, rest) = (seconds / HOUR, seconds % HOUR / MINUTE, seconds % MINUTE);
        let seconds_part = rest > 0 || nanos > 0 || (hours == 0 && minutes == 0);
        f.write_str("PT")?;
        if hours > 0 {
            write!(f, "{hours}H")?;
        }
        // Minutes always come between hours and seconds, even when they are
        // zero, so `PT1H0M5S` reads in the order RFC 3339's grammar expects.
        if minutes > 0 || (hours > 0 && seconds_part) {
            write!(f, "{minutes}M")?;
        }
        if !seconds_part {
            return Ok(());
        }
        write!(f, "{rest}")?;
        if nanos > 0 {
            // The fraction has nine digits, and each trailing zero comes off
            // with one digit of width, so `300_000_000` writes `3`.
            let (mut digits, mut width) = (nanos, 9);
            while digits % 10 == 0 {
                digits /= 10;
                width -= 1;
            }
            write!(f, ".{digits:0width$}")?;
        }
        f.write_str("S")
    }
}
