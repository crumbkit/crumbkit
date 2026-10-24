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

/// The type of a row field or a failure field that holds a length of time,
/// such as how long a request took. Text shows it as a compact duration,
/// such as `3h`, and JSON holds it as an ISO 8601 duration, such as
/// `PT3H25M`.
///
/// The compact duration is the largest whole unit only, the form that an
/// [`Age`](crate::Age) column prints: `{n}s` under a minute, `{n}m` under
/// an hour, `{n}h` under a day, `{n}d` under a week, and `{n}w` for a week
/// or more, so it reads `0s`, `59s`, `1m`, `3h`, `2d`, `1w` or `57w`. Each
/// unit rounds down, so 300 milliseconds is `0s` and 13 days is `1w`.
///
/// The JSON form is exact to the nanosecond. It is `PT`, then the whole
/// hours with `H`, the minutes from 1 to 59 with `M` and the seconds with
/// `S`, leaving out each part that is zero, such as `PT4M10S` or `PT1H5S`.
/// The seconds carry a fraction when the duration has one, with no
/// trailing zeros, such as `PT0.3S` or `PT1.000000001S`. A zero duration is
/// `PT0S`. Hours are the largest unit, so three days are `PT72H`, because
/// ISO 8601 reads a day as a calendar day, which a change of daylight
/// saving time can make 23 or 25 hours long. The field's JSON Schema is an
/// inline string in the format `duration`. RFC 3339's grammar for that
/// format, in its appendix A, has no fraction of a second, so a validator
/// that follows it strictly refuses a duration with a fraction, which ISO
/// 8601 itself allows.
///
/// `Duration` implements [`Shown`], so the text table, `--fields` text, a
/// stream's text line and [`Field::shown`](crate::Field::shown) print the
/// compact duration, and every JSON form holds the ISO 8601 duration.
///
/// ```
/// use crumbkit::Duration;
///
/// let took = Duration::from(std::time::Duration::from_secs(4 * 60 + 10));
///
/// assert_eq!(took.to_string(), "4m");
/// assert_eq!(serde_json::to_string(&took).unwrap(), r#""PT4M10S""#);
/// ```
//
// RUST-10: `Duration` serializes and has no `Deserialize`, because crumbkit
// prints documents and does not read them back. Reading the ISO 8601 form
// needs a parser for the general form, with days, weeks and fractions on
// any part, and the iso8601 crate or a parser of that size replaces this
// once a tool reads such documents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(pub std::time::Duration);

impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Self(duration)
    }
}

/// Writes the duration in the largest whole unit, such as `3h`.
impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        let (count, unit) = if seconds < MINUTE {
            (seconds, 's')
        } else if seconds < HOUR {
            (seconds / MINUTE, 'm')
        } else if seconds < DAY {
            (seconds / HOUR, 'h')
        } else if seconds < WEEK {
            (seconds / DAY, 'd')
        } else {
            (seconds / WEEK, 'w')
        };
        write!(f, "{count}{unit}")
    }
}

/// Writes the JSON form as an ISO 8601 duration in hours, minutes and
/// seconds, and describes it as an inline string in the `duration` format.
impl Shown for Duration {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&Iso8601(self.0))
    }

    fn schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A length of time as an ISO 8601 duration in hours, minutes and seconds, which the text table shows in its largest whole unit.",
            "type": "string",
            "format": "duration",
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
        f.write_str("PT")?;
        if hours > 0 {
            write!(f, "{hours}H")?;
        }
        if minutes > 0 {
            write!(f, "{minutes}M")?;
        }
        if rest == 0 && nanos == 0 && (hours > 0 || minutes > 0) {
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
