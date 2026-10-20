//! The age of a time in a row, which prints as a compact age in the text
//! table and as an RFC 3339 time in JSON.

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::ser::{Error as _, Serialize, Serializer};

#[cfg(test)]
mod tests;

/// The seconds in a minute.
const MINUTE: u64 = 60;

/// The seconds in an hour.
const HOUR: u64 = 60 * MINUTE;

/// The seconds in a day.
const DAY: u64 = 24 * HOUR;

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
/// `{n}m` under an hour, `{n}h` under a day, and `{n}d` for a day or more,
/// so it reads `0s`, `59s`, `1m`, `3h`, `2d` or `400d`. Each unit rounds
/// down. The largest unit is the day, because the length of a month or a
/// year depends on a calendar. `Age`'s `Display` writes the compact age, so
/// a row type that sets [`Row::LINE`](crate::Row::LINE) can put it in its
/// own `text_line`.
///
/// In JSON the field is the time in UTC with milliseconds, such as
/// `2026-10-07T12:01:07.120Z`. `--json`, `--json --fields`, `--json
/// --full` and the JSON of a stream all hold this form. The field's JSON Schema is a string with the
/// format `date-time`, so the row's schema lists it as a column that
/// `--fields` can name, and an `Option<Age>` that is `None` shows as `-` in
/// the table.
///
/// `Age`'s `Serialize` writes the compact age while crumbkit builds the
/// cells of a text table or of a stream's text line, and the RFC 3339 time
/// otherwise. crumbkit sets that mode on the current thread only, for the
/// duration of that serialization. It serializes a result's rows on the
/// thread that calls [`finish`](crate::finish) or
/// [`finish_to`](crate::finish_to), and a stream's rows on the thread that
/// calls [`Streaming::item`](crate::Streaming::item). A tool that
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
}

/// Writes the compact age, such as `3h`, in the largest whole unit.
impl fmt::Display for Age {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.elapsed.as_secs();
        let (count, unit) = if seconds < MINUTE {
            (seconds, 's')
        } else if seconds < HOUR {
            (seconds / MINUTE, 'm')
        } else if seconds < DAY {
            (seconds / HOUR, 'h')
        } else {
            (seconds / DAY, 'd')
        };
        write!(f, "{count}{unit}")
    }
}

/// Serializes the compact age while crumbkit builds text cells on this
/// thread, and the time in RFC 3339 with milliseconds otherwise.
impl Serialize for Age {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if text_cells() {
            return serializer.collect_str(self);
        }
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
}

/// Describes the age as a string in the `date-time` format. The schema is
/// inline, so a row's property holds it with the field's own description,
/// and an `Option<Age>` is a string or null.
impl JsonSchema for Age {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "Age".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::Age".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A time in RFC 3339 in UTC with milliseconds, which the text table shows as its age.",
            "type": "string",
            "format": "date-time",
        })
    }
}

thread_local! {
    /// Whether an [`Age`] on this thread serializes as its compact age,
    /// which [`TextCells`] sets while crumbkit builds text cells.
    static TEXT_CELLS: Cell<bool> = const { Cell::new(false) };
}

/// Reports whether this thread is building text cells.
fn text_cells() -> bool {
    TEXT_CELLS.try_with(Cell::get).unwrap_or(false)
}

/// The text mode of this thread, which makes an [`Age`] serialize as its
/// compact age until the guard is dropped. Dropping it restores the mode
/// that [`TextCells::enter`] found, so a guard inside another leaves the
/// outer one in effect, and a panic during the serialization ends the mode
/// too.
///
/// crumbkit holds one only around the serialization of rows whose value
/// becomes the cells of a text table or of a stream's text line.
#[must_use = "the text mode ends when the guard is dropped"]
pub(crate) struct TextCells {
    /// The mode that the thread had before this guard.
    previous: bool,
}

impl TextCells {
    /// Sets the text mode on this thread until the guard is dropped.
    pub(crate) fn enter() -> Self {
        let previous = TEXT_CELLS
            .try_with(|text| text.replace(true))
            .unwrap_or(false);
        Self { previous }
    }
}

impl Drop for TextCells {
    fn drop(&mut self) {
        // The thread-local holds a `bool`, so it has no destructor and the
        // access fails only while the thread itself is being torn down.
        let _ = TEXT_CELLS.try_with(|text| text.set(self.previous));
    }
}
