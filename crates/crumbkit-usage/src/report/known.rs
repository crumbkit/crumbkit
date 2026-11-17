//! The check that a record names only what the tool and the usage log can
//! write, which the reader makes before it adds a record to the report.
//!
//! The usage log is a file that anyone with the user's rights can edit, so
//! a line may hold any text. The report prints command paths, flags, codes,
//! callers, and the trace, run and parent span ids, and leaves out runs by
//! their program, and the check keeps
//! every value that the writer could not have produced out of the report
//! (ADR-0029).

use clap::{Arg, Command};

use super::entry::Entry;
use crate::facts::is_label;
use crate::program::is_program;

#[cfg(test)]
mod tests;

/// The callers that the usage log writes besides an agent's id.
const OTHER_CALLERS: [&str; 2] = ["person", "unknown"];

/// The longest code that the check keeps, in bytes, which are ASCII
/// characters. A longer code is skipped.
const LONGEST_CODE: usize = 128;

/// What one line of the usage log is, for a reader that holds a record only
/// once the check has passed.
pub(crate) enum Read {
    /// The line is not a whole usage record, such as the last line of a run
    /// that ended while it wrote.
    NotRecord,
    /// The line is a usage record that names something the writer could not
    /// have produced, with the time it says the run ended, in milliseconds
    /// since the Unix epoch.
    Unknown(u64),
    /// The line is a record that [`known`] accepts.
    Known(Known),
}

/// An [`Entry`] that [`known`] accepted. Only [`read`] makes one, so a
/// reader that holds a `Known` holds a record the writer could have
/// produced.
pub(crate) struct Known(Entry);

impl Known {
    /// Returns the checked record.
    pub(crate) fn entry(&self) -> &Entry {
        &self.0
    }

    /// Returns the checked record, which the caller then owns.
    pub(crate) fn into_entry(self) -> Entry {
        self.0
    }
}

/// Reads `line` and checks it against `tool`, a command that
/// `Command::build` has prepared, as [`known`] describes.
pub(crate) fn read(tool: &Command, line: &[u8]) -> Read {
    match Entry::parse(line) {
        None => Read::NotRecord,
        Some(entry) if known(tool, &entry) => Read::Known(Known(entry)),
        Some(entry) => Read::Unknown(entry.at),
    }
}

/// Returns whether every name in `entry` is one that the writer could have
/// produced for `tool`, a command that `Command::build` has prepared:
///
/// - the command path names subcommands of `tool` by their real names, one
///   level below the other;
/// - each flag and the nearest flag is the name of an argument of `tool` or
///   of a command on the path, as the writer names it: `--long`, or `-s`
///   for an argument without a long name;
/// - the code and each tolerated code is 1 to 128 ASCII letters, digits, `_`,
///   `-` and `:`, which covers crumbkit's spelling of a
///   [`Code`](crumbkit::Code), and keeps out whitespace and any other
///   character;
/// - the caller is an agent id from is-ai-agent, `person` or `unknown`;
/// - the program, when there is one, is 1 to 32 lowercase ASCII letters,
///   digits, `-` and `_`, starting with a letter;
/// - the context, when there is one, is a [`Label`](crate::Label): 1 to 64
///   ASCII letters, digits, braces and `_-./:`;
/// - the trace id is 32 lowercase hex digits, and the run's and the
///   parent's span ids are 16, as the writer formats them (ADR-0030).
pub(super) fn known(tool: &Command, entry: &Entry) -> bool {
    let Some(path) = path(tool, entry.command.as_deref()) else {
        return false;
    };
    let flag = |flag: &String| {
        path.iter()
            .any(|command| command.get_arguments().any(|arg| names(arg, flag)))
    };
    entry.flags.iter().all(flag)
        && entry.nearest.as_ref().is_none_or(flag)
        && entry.code.as_deref().is_none_or(is_code)
        && entry.tolerated.iter().all(|code| is_code(code))
        && is_caller(&entry.caller)
        && entry.program.as_deref().is_none_or(is_program)
        && entry.context.as_deref().is_none_or(is_label)
        && is_hex(&entry.trace, 32)
        && is_hex(&entry.run, 16)
        && entry
            .parent
            .as_deref()
            .is_none_or(|parent| is_hex(parent, 16))
}

/// Returns whether `text` is `digits` lowercase hex digits.
pub(crate) fn is_hex(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Returns `tool` and the subcommands that `command` names below it by
/// their real names, or `None` when a name is not a subcommand at its
/// level. `None` for `command` is the root alone.
fn path<'a>(tool: &'a Command, command: Option<&str>) -> Option<Vec<&'a Command>> {
    let mut path = vec![tool];
    let Some(command) = command else {
        return Some(path);
    };
    let mut node = tool;
    for name in command.split(' ') {
        node = node
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == name)?;
        path.push(node);
    }
    Some(path)
}

/// Returns whether `flag` is the name that the writer gives `arg`.
fn names(arg: &Arg, flag: &str) -> bool {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => flag.strip_prefix("--") == Some(long),
        (None, Some(short)) => {
            let mut chars = flag.chars();
            chars.next() == Some('-') && chars.next() == Some(short) && chars.next().is_none()
        }
        (None, None) => false,
    }
}

/// Returns whether `text` is 1 to [`LONGEST_CODE`] ASCII letters, digits,
/// `_`, `-` and `:`.
fn is_code(text: &str) -> bool {
    (1..=LONGEST_CODE).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-:".contains(&byte))
}

/// Returns whether the usage log writes `name` as a caller. An agent's id
/// is checked by asking is-ai-agent to detect an agent from `AGENT` set to
/// the name, which gives the agent back only for the id that the agent
/// writes.
fn is_caller(name: &str) -> bool {
    OTHER_CALLERS.contains(&name)
        || is_ai_agent::detect_with(|var| (var == "AGENT").then(|| name.to_owned()), |_| false)
            .is_some_and(|agent| agent.id.as_str() == name)
}
