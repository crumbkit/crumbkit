//! The rows that `completion install` and `completion check` print, and the
//! script that `completion print` prints.
//!
//! The `Serialize` and `JsonSchema` impls of this module are written by
//! hand, so crumbkit needs no `serde_derive` and no schemars `derive`
//! feature.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};

use super::shell::Shell;
use crate::{Problem, Row};

/// Returns the schema of a row's `shell` field, with the description that
/// a leaf's help shows for the column.
fn shell_property() -> serde_json::Value {
    serde_json::json!({
        "description": "The shell: bash, elvish, fish, powershell or zsh.",
        "enum": ["bash", "elvish", "fish", "powershell", "zsh"],
    })
}

/// What install did with the completion file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Installed {
    /// Install could not read or write the file.
    Failed,
    /// Install left a file or a link that it did not write.
    Skipped,
    /// Install wrote the file.
    Written,
    /// The file already held this script.
    Unchanged,
}

impl Installed {
    /// Returns the state's name in the row, such as `written`.
    const fn name(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Written => "written",
            Self::Unchanged => "unchanged",
        }
    }
}

/// The completion file of one shell, and what install did with it.
///
/// The text table shows the state, the shell, the file's path and the
/// reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallRow {
    // The `JsonSchema` impl below describes each field.
    pub(crate) state: Installed,
    pub(crate) shell: Shell,
    pub(crate) path: String,
    pub(crate) reason: Option<String>,
}

impl Row for InstallRow {
    const COLUMNS: &'static [&'static str] = &["state", "shell", "path", "reason"];

    // A failed file makes install exit with 1. A skipped one does not,
    // because the person or program that owns that file chose it.
    const PROBLEM: Option<Problem<Self>> =
        Some(Problem::by("state", |row| row.state == Installed::Failed));
}

impl Serialize for InstallRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("InstallRow", 4)?;
        row.serialize_field("state", self.state.name())?;
        row.serialize_field("shell", &self.shell)?;
        row.serialize_field("path", &self.path)?;
        row.serialize_field("reason", &self.reason)?;
        row.end()
    }
}

impl JsonSchema for InstallRow {
    fn schema_name() -> Cow<'static, str> {
        "InstallRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::completion::InstallRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The completion file of one shell, and what install did with it.",
            "type": "object",
            "properties": {
                "state": {
                    "description": "What install did: failed, skipped, written or unchanged.",
                    "enum": ["failed", "skipped", "written", "unchanged"],
                },
                "shell": shell_property(),
                "path": {
                    "description": "The completion file that install wrote or left. When the file is a link to a file that install wrote, the path of that file.",
                    "type": "string",
                },
                "reason": {
                    "description": "Why install skipped or failed, or what it replaced under --force, such as `linked to /src/shelf/shelf.fish`, or null.",
                    "type": ["string", "null"],
                },
            },
            "required": ["state", "shell", "path", "reason"],
        })
    }
}

/// Whether the completion file holds the script that install writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Checked {
    /// No completion file exists.
    Missing,
    /// The file differs from the script that install writes.
    Differs,
    /// The file cannot be read.
    Unreadable,
    /// The file is a link to a file that install did not write, which
    /// install skips.
    Linked,
    /// The file holds the script that install writes.
    Current,
}

impl Checked {
    /// Returns the state's name in the row, such as `missing`.
    const fn name(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Differs => "differs",
            Self::Unreadable => "unreadable",
            Self::Linked => "linked",
            Self::Current => "current",
        }
    }
}

/// The completion file of one shell, and whether it holds the script that
/// install writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRow {
    // The `JsonSchema` impl below describes each field.
    pub(crate) state: Checked,
    pub(crate) shell: Shell,
    pub(crate) path: String,
    pub(crate) reason: Option<String>,
}

impl Row for CheckRow {
    const COLUMNS: &'static [&'static str] = &["state", "shell", "path", "reason"];

    // A file that is missing, differs or cannot be read makes check exit
    // with 1, and the document lists only those rows in text. A linked file
    // does not, because install skips it and its owner chose it.
    const PROBLEM: Option<Problem<Self>> = Some(Problem::by("state", |row| {
        !matches!(row.state, Checked::Current | Checked::Linked)
    }));
}

impl Serialize for CheckRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_struct("CheckRow", 4)?;
        row.serialize_field("state", self.state.name())?;
        row.serialize_field("shell", &self.shell)?;
        row.serialize_field("path", &self.path)?;
        row.serialize_field("reason", &self.reason)?;
        row.end()
    }
}

impl JsonSchema for CheckRow {
    fn schema_name() -> Cow<'static, str> {
        "CheckRow".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::completion::CheckRow".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The completion file of one shell, and whether it holds the script that install writes.",
            "type": "object",
            "properties": {
                "state": {
                    "description": "Whether the file holds the script that install writes: missing, differs, unreadable, linked or current.",
                    "enum": ["missing", "differs", "unreadable", "linked", "current"],
                },
                "shell": shell_property(),
                "path": {
                    "description": "The completion file that was checked. When the file is a link to a file that install wrote, the path of that file.",
                    "type": "string",
                },
                "reason": {
                    "description": "Why the file differs, cannot be read or is linked, such as `linked to /src/shelf/shelf.fish`, or null.",
                    "type": ["string", "null"],
                },
            },
            "required": ["state", "shell", "path", "reason"],
        })
    }
}

/// A shell's registration script, as `completion print` prints it. The text
/// output is the script, and the JSON document holds the shell beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    // The `JsonSchema` impl below describes each field.
    pub(crate) shell: Shell,
    pub(crate) text: String,
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl Serialize for Script {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut script = serializer.serialize_struct("Script", 2)?;
        script.serialize_field("shell", &self.shell)?;
        script.serialize_field("text", &self.text)?;
        script.end()
    }
}

impl JsonSchema for Script {
    fn schema_name() -> Cow<'static, str> {
        "Script".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::completion::Script".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A shell's registration script for the tool's completion.",
            "type": "object",
            "properties": {
                "shell": shell_property(),
                "text": {
                    "description": "The registration script, which the shell runs once to register the tool's completion.",
                    "type": "string",
                },
            },
            "required": ["shell", "text"],
        })
    }
}
