//! The age of a time in a row, which prints as a compact age in the text
//! table and as an RFC 3339 time in JSON.

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use schemars::{Schema, SchemaGenerator, json_schema};
use serde::ser::{Error as _, Serializer};

use crate::Shown;

#[cfg(test)]
mod tests;

/// The seconds from 1970 to the start of the year 10000. RFC 3339 writes a
/// year in four digits, so a time from this second on has no RFC 3339 form.
const YEAR_10000: u64 = 253_402_300_800;

/// The type of a row field that holds the time something happened, such as
/// when a file changed. The text table shows how long ago that was, as a
/// compact age such as `3h`, and JSON holds the time itself in RFC 3339.
///
/// [`Age::since`] takes the time and the current time from the tool, so
/// crumbkit reads no clock, and a test or a golden file that passes a fixed
/// current time prints the same age on every run.
///
/// The compact age is the largest whole unit only: `{n}s` under a minute,
/// `{n}m` under an hour, `{n}h` under a day, `{n}d` under a week, and
/// `{n}w` for a week or more, so it reads `0s`, `59s`, `1m`, `3h`, `2d`,
/// `1w` or `57w`. Each unit rounds down, so 13 days is `1w`. The largest
/// unit is the week, which is always seven days, because the length of a
/// month or a year depends on a calendar. `Age`'s `Display` writes the
/// compact age, so a row type that sets [`Row::LINE`](crate::Row::LINE)
/// can put it in its own `text_line`. [`Age::compact`] writes the same
/// compact age for a duration that the tool measured.
///
/// In JSON the field is the time in UTC with milliseconds, such as
/// `2026-10-07T12:01:07.120Z`. `--json`, `--json --fields`, `--json --full`
/// and the JSON of a stream all hold this form. The field's JSON Schema is
/// a string with the format `date-time`, so the row's schema lists it as a
/// column that `--fields` can name, and an `Option<Age>` that is `None`
/// shows as `-` in the table.
///
/// `Age` implements [`Shown`], so crumbkit prints the compact age where it
/// builds text cells and the RFC 3339 time in every JSON form, as
/// [`shown::serialize`](crate::shown::serialize) describes. A tool that
/// serializes a row itself, with `serde_json` or another serializer, always
/// gets the RFC 3339 time.
///
/// A time before 1970 or from the year 10000 on has no RFC 3339 form in four
/// digits, so its JSON fails to serialize and crumbkit reports a failed
/// write. Its compact age still prints in text.
///
/// ```
/// use std::time::{Duration, UNIX_EPOCH};
///
/// use crumbkit::Age;
///
/// let changed = UNIX_EPOCH + Duration::from_millis(1_791_374_467_120);
/// let now = changed + Duration::from_secs(3 * 60 * 60 + 25 * 60);
/// let age = Age::since(changed, now);
///
/// assert_eq!(age.to_string(), "3h");
/// assert_eq!(
///     serde_json::to_string(&age).unwrap(),
///     r#""2026-10-07T12:01:07.120Z""#
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Age {
    /// The time that the age counts from.
    at: SystemTime,
    /// The duration from `at` to the current time that the tool passed.
    elapsed: Duration,
}

impl Age {
    /// Returns the age of `at` when the current time is `now`. The age is
    /// zero when `at` is later than `now`, as it is for a file whose clock
    /// runs ahead of the tool's.
    #[must_use]
    pub fn since(at: SystemTime, now: SystemTime) -> Self {
        let elapsed = now.duration_since(at).unwrap_or_default();
        Self { at, elapsed }
    }

    /// Returns the time that the age counts from, which JSON holds.
    #[must_use]
    pub const fn at(&self) -> SystemTime {
        self.at
    }

    /// Returns the duration from the age's time to the current time that
    /// [`Age::since`] took, which the compact age shows.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Returns a value whose `Display` writes `elapsed` as a compact age, in
    /// the same units and with the same boundaries as `Age`'s `Display`.
    ///
    /// It is for a duration that the tool measured, such as with
    /// [`Instant::elapsed`](std::time::Instant::elapsed), when the tool has
    /// no time that the duration counts from. A row field that JSON should
    /// hold as a length of time is a [`Duration`](crate::Duration), whose
    /// text form shows two units, such as `4m 10s`.
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// use crumbkit::Age;
    ///
    /// let waited = Duration::from_millis(3_400);
    /// assert_eq!(Age::compact(waited).to_string(), "3s");
    ///
    /// let idle = Duration::from_secs(15 * 24 * 60 * 60);
    /// assert_eq!(Age::compact(idle).to_string(), "2w");
    /// ```
    #[must_use]
    pub const fn compact(elapsed: Duration) -> impl fmt::Display {
        crate::duration::Largest(elapsed)
    }
}

/// Writes the compact age, such as `3h`, in the largest whole unit.
impl fmt::Display for Age {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::duration::Largest(self.elapsed).fmt(f)
    }
}

/// Writes the JSON form as the time in RFC 3339 with milliseconds, and
/// describes it as an inline string in the `date-time` format.
impl Shown for Age {
    fn json<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // humantime panics on a time before 1970 and returns `fmt::Error`
        // from the year 10000 on, and serde_json panics on a `Display` that
        // fails, so both are refused here first.
        let since_1970 = self
            .at
            .duration_since(UNIX_EPOCH)
            .map_err(|_| S::Error::custom("an Age before 1970 has no RFC 3339 form"))?;
        if since_1970.as_secs() >= YEAR_10000 {
            return Err(S::Error::custom(
                "an Age after the year 9999 has no RFC 3339 form",
            ));
        }
        serializer.collect_str(&humantime::format_rfc3339_millis(self.at))
    }

    fn schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A time in RFC 3339 in UTC with milliseconds, which the text table shows as its age.",
            "type": "string",
            "format": "date-time",
        })
    }
}

crate::shown!(Age);
