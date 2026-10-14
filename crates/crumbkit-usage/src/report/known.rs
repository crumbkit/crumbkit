//! The check that a record names only what the tool and the usage log can
//! write, which the reader makes before it adds a record to the report.
//!
//! The usage log is a file that anyone with the user's rights can edit, so
//! a line may hold any text. The report prints command paths, flags, codes
//! and callers, and the check keeps every value that the writer could not
//! have produced out of the report (ADR-0029).

use clap::{Arg, Command};

use super::entry::Entry;

#[cfg(test)]
mod tests;

/// The callers that the usage log writes besides an agent's id.
const OTHER_CALLERS: [&str; 2] = ["person", "unknown"];

/// The longest code that the check keeps, in bytes, which are ASCII
/// characters. A longer code is skipped.
const LONGEST_CODE: usize = 128;

/// Returns whether every name in `entry` is one that the writer could have
/// produced for `tool`, a command that `Command::build` has prepared:
///
/// - the command path names subcommands of `tool` by their real names, one
///   level below the other;
/// - each flag and the nearest flag is the name of an argument of `tool` or
///   of a command on the path, as the writer names it: `--long`, or `-s`
///   for an argument without a long name;
/// - the code and each tolerated code is 1 to 128 ASCII letters, digits, `_`, `-`, `:`, `.` and
///   `/`, which covers crumbkit's predefined spellings of a
///   [`Code`](crate::Code) and a tool's custom spelling, and keeps out
///   whitespace and any other character;
/// - the caller is an agent id from is-ai-agent, `person` or `unknown`.
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
/// `_`, `-`, `:`, `.` and `/`.
fn is_code(text: &str) -> bool {
    (1..=LONGEST_CODE).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-:./".contains(&byte))
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
