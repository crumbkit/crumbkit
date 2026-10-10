//! The record of one run, which the usage log holds as one line of JSON.
//!
//! Every field is a name from the tool's own vocabulary, a count, a flag, a
//! time or a keyed hash, so no field can hold an argument's value. The
//! app's section is the one exception that crumbkit cannot check
//! (ADR-0029).

use std::hash::Hasher as _;

use schemars::JsonSchema;
use serde::Serialize;
use siphasher::sip::SipHasher24;

#[cfg(test)]
mod tests;

/// The id of the record, which every line holds under `schema`.
pub(super) const SCHEMA: &str = "crumbkit/usage/v1";

/// The longest line, in bytes without its newline, that the usage log
/// writes.
const MAX_LINE: usize = 1024;

/// One run of a tool as its usage log records it.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(
    title = "UsageRecord",
    description = "One line of a tool's usage log, which records the shape of one run and never the values of its arguments."
)]
pub(super) struct Record {
    /// The id of this record.
    #[schemars(extend("const" = "crumbkit/usage/v1"))]
    pub(super) schema: &'static str,
    /// The name of the tool's root command, such as shelf.
    pub(super) tool: Name,
    /// The tool's version, when its command has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Name")]
    pub(super) version: Option<Name>,
    /// When the run ended, in RFC 3339 with milliseconds, such as
    /// 2026-10-07T12:01:07.120Z.
    pub(super) at: String,
    /// The run's wall time in milliseconds, from `start` to the line.
    pub(super) ms: u64,
    /// The id of the coding agent that ran the tool, such as claude-code;
    /// person when no agent is detected and standard input is a terminal;
    /// and unknown otherwise.
    pub(super) caller: &'static str,
    /// The keyed hash of the agent's session id, or of the parent process id
    /// when the agent gives no session id.
    pub(super) session: Hash,
    /// The keyed hash of the words after the tool's name, so identical
    /// command lines share a hash.
    pub(super) args: Hash,
    /// Whether the line was written when the run ended on SIGINT or SIGTERM.
    /// It is present only when that is so.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) interrupted: bool,
    /// What `parse`, `finish`, an alias and the tool reported during the
    /// run.
    #[serde(flatten)]
    pub(super) run: Run,
}

/// What `parse`, `finish`, an alias and the tool report during a run. A
/// field that nothing reported is left out of the line.
#[derive(Debug, Default, Serialize, JsonSchema)]
pub(super) struct Run {
    /// The command path below the tool's name, such as books show.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Name")]
    pub(super) command: Option<Name>,
    /// short for -h, and long for --help or the help command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Help")]
    pub(super) help: Option<Help>,
    /// The flags typed on the command line, each by its long name, or by its
    /// short name when it has no long one, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) flags: Vec<Name>,
    /// How many values the command's positional arguments named ID received.
    /// It is present only when the command has such an argument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub(super) ids: Option<usize>,
    /// How many flags of a refused command line name no flag of the command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub(super) unknown_flags: Option<usize>,
    /// The real flag that clap suggested for a refused flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Name")]
    pub(super) nearest: Option<Name>,
    /// Whether a personal alias expanded. It is present only when one did.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) alias: bool,
    /// The exit code that `parse` or `finish` returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u8")]
    pub(super) exit: Option<u8>,
    /// The code of the run's failure or refusal in the tool's spelling, such
    /// as `shelf::books::not_found`. A batch gives the code of its first
    /// failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Name")]
    pub(super) code: Option<Name>,
    /// The bytes that `finish` wrote to standard output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub(super) out_bytes: Option<u64>,
    /// Whether a listing showed fewer rows than its total. It is present only
    /// when one did.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) truncated: bool,
    /// The tool's own section, when the tool sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Section")]
    pub(super) ext: Option<Section>,
}

/// A name from the tool's own vocabulary: a command path, a flag, a version
/// or a failure code.
///
/// crumbkit builds every name from the tool's command tree and its codes,
/// and never from a word that the caller typed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(transparent)]
pub(super) struct Name(String);

impl Name {
    /// Returns the name `text`, which the caller read from the tool's
    /// command tree or its codes.
    pub(super) fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// Returns the name as text.
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The keyed SipHash-2-4 hash of a list of byte strings, as 16 lowercase hex
/// digits.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("pattern" = "^[0-9a-f]{16}$"))]
pub(super) struct Hash(String);

impl Hash {
    /// Hashes `parts` with `key`. Each part's length goes in before its
    /// bytes, so `["ab", "c"]` and `["a", "bc"]` hash differently.
    pub(super) fn new<'a>(key: &[u8; 16], parts: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut hasher = SipHasher24::new_with_key(key);
        for part in parts {
            hasher.write(&(part.len() as u64).to_le_bytes());
            hasher.write(part);
        }
        Self(format!("{:016x}", hasher.finish()))
    }
}

/// Which help page the caller asked for.
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(super) enum Help {
    /// `-h`.
    Short,
    /// `--help`, or the help command.
    Long,
}

/// The tool's own section of the record, which it sets with
/// `crumbkit::usage::section`.
#[derive(Debug, Serialize, JsonSchema)]
pub(super) struct Section {
    /// The id of the section's document, such as shelf/usage-ext/v1.
    pub(super) schema: String,
    /// The section's fields. The tool owns them and keeps argument values out
    /// of them, because crumbkit cannot check them.
    pub(super) data: serde_json::Value,
}

/// Returns `record` as one line of JSON that ends with a newline, or `None`
/// when the line would be longer than 1 KiB without its newline or cannot
/// be serialized.
pub(super) fn line(record: &Record) -> Option<Vec<u8>> {
    let mut line = serde_json::to_vec(record).ok()?;
    if line.len() > MAX_LINE {
        return None;
    }
    line.push(b'\n');
    Some(line)
}

/// Returns whether `value` is false, so serde leaves out a false flag.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes the field by reference"
)]
const fn is_false(value: &bool) -> bool {
    !*value
}
