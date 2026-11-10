//! One line of the usage log as the report reads it.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Deserializer};

use crate::record::SCHEMA;

#[cfg(test)]
mod tests;

/// The fields of one `<tool>/usage/v1` record that the report reads.
///
/// The reader ignores the fields that it does not name here, as every
/// reader of a crumbkit document ignores fields it does not know (ADR-0007).
/// The tool's name and version are never read, and of the tool's own facts
/// only the time spent waiting is read, so the report cannot print them.
/// `self runs list` prints the trace ids, which [`known`](super::known)
/// checks as lowercase hex first.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Entry {
    /// The record's schema id, which must be `<tool>/usage/v1`.
    schema: String,
    /// When the run ended, in milliseconds since the Unix epoch.
    #[serde(deserialize_with = "millis")]
    pub(crate) at: u64,
    /// The run's wall time in milliseconds.
    pub(crate) ms: u64,
    /// The coding agent that ran the tool, `person` or `unknown`.
    pub(crate) caller: String,
    /// The keyed hash of the run's session, which groups the runs of one
    /// session and is never printed.
    pub(crate) session: String,
    /// The keyed hash of the run's command line, which is never printed.
    pub(crate) args: String,
    /// The id of the W3C trace that the run belongs to, which `self runs
    /// list` prints once it is 32 lowercase hex digits.
    pub(crate) trace: String,
    /// The run's own span id, which `self runs list` prints once it is 16
    /// lowercase hex digits.
    pub(crate) run: String,
    /// The caller's span id, when the run continued a trace.
    #[serde(default)]
    pub(crate) parent: Option<String>,
    /// The command path below the tool's name, such as `books show`.
    #[serde(default)]
    pub(crate) command: Option<String>,
    /// Whether the run printed a help page.
    #[serde(default, deserialize_with = "present")]
    pub(crate) help: bool,
    /// The flags that the caller typed.
    #[serde(default)]
    pub(crate) flags: Vec<String>,
    /// How many ids the command's positional arguments received.
    #[serde(default)]
    pub(crate) ids: Option<u64>,
    /// The real flag that clap suggested for a refused flag.
    #[serde(default)]
    pub(crate) nearest: Option<String>,
    /// The run's exit code.
    #[serde(default)]
    pub(crate) exit: Option<u8>,
    /// The code of the run's failure or refusal.
    #[serde(default)]
    pub(crate) code: Option<String>,
    /// The codes of the failures that the run reported and carried on
    /// past.
    #[serde(default)]
    pub(crate) tolerated: Vec<String>,
    /// The bytes that the run wrote to standard output.
    #[serde(default)]
    pub(crate) out_bytes: Option<u64>,
    /// Whether a listing showed fewer rows than its total.
    #[serde(default)]
    pub(crate) truncated: bool,
    /// Whether the run ended on SIGINT or SIGTERM.
    #[serde(default)]
    pub(crate) interrupted: bool,
    /// The one part of the tool's own facts that the report reads.
    #[serde(default)]
    pub(crate) facts: Facts,
}

/// The part of a record's `facts` that the report reads: the time that the
/// run spent waiting on another system (ADR-0145). The report never reads
/// the tool's counts or labels, so it cannot print their names or text.
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Facts {
    /// The milliseconds that the run spent waiting, or 0 when the tool
    /// reported none.
    #[serde(default)]
    pub(crate) waited_ms: u64,
}

impl Entry {
    /// Returns the record that `line` holds, or `None` when the line is not
    /// a usage record, such as `shelf/usage/v1`, with every field that each
    /// record has, such as a line that a run cut short. The id's component
    /// is the name of the tool that wrote the log, so any component counts.
    pub(crate) fn parse(line: &[u8]) -> Option<Self> {
        let entry: Self = serde_json::from_slice(line).ok()?;
        let suffix = SCHEMA.split_once('/').map(|(_, suffix)| suffix);
        let read = entry.schema.split_once('/').map(|(_, suffix)| suffix);
        (read.is_some() && read == suffix).then_some(entry)
    }

    /// Returns whether the run did what the caller asked: it exited with 0,
    /// printed no help page and was not interrupted.
    pub(crate) fn succeeded(&self) -> bool {
        self.exit == Some(0) && !self.help && !self.interrupted
    }

    /// Returns the code of the run's failure when the command failed, which
    /// exits with 1 (ADR-0002), or `None` otherwise.
    pub(crate) fn failure(&self) -> Option<&str> {
        if self.exit == Some(1) && !self.interrupted {
            self.code.as_deref()
        } else {
            None
        }
    }

    /// Returns whether the caller typed `flag`.
    pub(crate) fn typed(&self, flag: &str) -> bool {
        self.flags.iter().any(|typed| typed == flag)
    }
}

/// Reads an RFC 3339 time, such as `2026-10-07T12:01:07.120Z`, as
/// milliseconds since the Unix epoch.
fn millis<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let text = String::deserialize(deserializer)?;
    millis_of(&text).ok_or_else(|| serde::de::Error::custom("not an RFC 3339 time after 1970"))
}

/// Returns the RFC 3339 time `text`, such as `2026-10-07T12:01:07.120Z`, as
/// milliseconds since the Unix epoch, or `None` when it is not such a time
/// or is before 1970.
pub(crate) fn millis_of(text: &str) -> Option<u64> {
    let time = humantime::parse_rfc3339(text).ok()?;
    let since_epoch = time.duration_since(UNIX_EPOCH).ok()?;
    u64::try_from(since_epoch.as_millis()).ok()
}

/// Reads a field that is present with any string value as `true`.
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    String::deserialize(deserializer).map(|_| true)
}

#[cfg(test)]
impl Entry {
    /// Returns the record of a person's run of `command` in `session` that
    /// ended `at` seconds after the Unix epoch, took 10 ms, typed no flag
    /// and exited with 0. A test changes the fields it needs.
    pub(crate) fn run(session: &str, at: u64, command: &str) -> Self {
        Self {
            schema: SCHEMA.to_owned(),
            at: at * 1000,
            ms: 10,
            caller: "person".to_owned(),
            session: session.to_owned(),
            args: format!("{command} {at}"),
            trace: "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
            run: "00f067aa0ba902b7".to_owned(),
            parent: None,
            command: (!command.is_empty()).then(|| command.to_owned()),
            help: false,
            flags: Vec::new(),
            ids: None,
            nearest: None,
            exit: Some(0),
            code: None,
            tolerated: Vec::new(),
            out_bytes: Some(100),
            truncated: false,
            interrupted: false,
            facts: Facts::default(),
        }
    }
}
