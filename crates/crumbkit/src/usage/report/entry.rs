//! One line of the usage log as the report reads it.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Deserializer};

use crate::usage::record::SCHEMA;

#[cfg(test)]
mod tests;

/// The fields of one `crumbkit/usage/v1` record that the report reads.
///
/// The reader ignores the fields that it does not name here, as every
/// reader of a crumbkit document ignores fields it does not know (ADR-0007).
/// The trace ids, the tool's name and version and the tool's own section
/// are never read, so the report cannot print them.
#[derive(Clone, Debug, Deserialize)]
pub(super) struct Entry {
    /// The record's schema id, which must be `crumbkit/usage/v1`.
    schema: String,
    /// When the run ended, in milliseconds since the Unix epoch.
    #[serde(deserialize_with = "millis")]
    pub(super) at: u64,
    /// The run's wall time in milliseconds.
    pub(super) ms: u64,
    /// The coding agent that ran the tool, `person` or `unknown`.
    pub(super) caller: String,
    /// The keyed hash of the run's session, which groups the runs of one
    /// session and is never printed.
    pub(super) session: String,
    /// The keyed hash of the run's command line, which is never printed.
    pub(super) args: String,
    /// The command path below the tool's name, such as `books show`.
    #[serde(default)]
    pub(super) command: Option<String>,
    /// Whether the run printed a help page.
    #[serde(default, deserialize_with = "present")]
    pub(super) help: bool,
    /// The flags that the caller typed.
    #[serde(default)]
    pub(super) flags: Vec<String>,
    /// How many ids the command's positional arguments received.
    #[serde(default)]
    pub(super) ids: Option<u64>,
    /// The real flag that clap suggested for a refused flag.
    #[serde(default)]
    pub(super) nearest: Option<String>,
    /// The run's exit code.
    #[serde(default)]
    pub(super) exit: Option<u8>,
    /// The code of the run's failure or refusal.
    #[serde(default)]
    pub(super) code: Option<String>,
    /// The codes of the failures that the run reported and carried on
    /// past, in the tool's spelling.
    #[serde(default)]
    pub(super) tolerated: Vec<String>,
    /// The bytes that the run wrote to standard output.
    #[serde(default)]
    pub(super) out_bytes: Option<u64>,
    /// Whether a listing showed fewer rows than its total.
    #[serde(default)]
    pub(super) truncated: bool,
    /// Whether the run ended on SIGINT or SIGTERM.
    #[serde(default)]
    pub(super) interrupted: bool,
}

impl Entry {
    /// Returns the record that `line` holds, or `None` when the line is not
    /// a `crumbkit/usage/v1` record with every field that each record has,
    /// such as a line that a run cut short.
    pub(super) fn parse(line: &[u8]) -> Option<Self> {
        let entry: Self = serde_json::from_slice(line).ok()?;
        (entry.schema == SCHEMA).then_some(entry)
    }

    /// Returns whether the run did what the caller asked: it exited with 0,
    /// printed no help page and was not interrupted.
    pub(super) fn succeeded(&self) -> bool {
        self.exit == Some(0) && !self.help && !self.interrupted
    }

    /// Returns whether clap or `--fields` refused the run's command line,
    /// which exits with 2 (ADR-0002).
    pub(super) fn refused(&self) -> bool {
        self.exit == Some(2)
    }

    /// Returns the code of the run's failure when the command failed, which
    /// exits with 1 (ADR-0002), or `None` otherwise.
    pub(super) fn failure(&self) -> Option<&str> {
        if self.exit == Some(1) && !self.interrupted {
            self.code.as_deref()
        } else {
            None
        }
    }

    /// Returns whether the caller typed `flag`.
    pub(super) fn typed(&self, flag: &str) -> bool {
        self.flags.iter().any(|typed| typed == flag)
    }
}

/// Reads an RFC 3339 time, such as `2026-10-07T12:01:07.120Z`, as
/// milliseconds since the Unix epoch.
fn millis<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let text = String::deserialize(deserializer)?;
    let time = humantime::parse_rfc3339(&text).map_err(serde::de::Error::custom)?;
    let since_epoch = time
        .duration_since(UNIX_EPOCH)
        .map_err(serde::de::Error::custom)?;
    u64::try_from(since_epoch.as_millis()).map_err(serde::de::Error::custom)
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
    pub(super) fn run(session: &str, at: u64, command: &str) -> Self {
        Self {
            schema: SCHEMA.to_owned(),
            at: at * 1000,
            ms: 10,
            caller: "person".to_owned(),
            session: session.to_owned(),
            args: format!("{command} {at}"),
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
        }
    }
}
