//! Exit codes that a tool declares in its guide, which `<tool> help
//! exit-codes` and `commands list` describe.

use std::fmt::Write as _;

use super::Guide;
use crate::Code;

#[cfg(test)]
mod tests;

/// The name of the help topic that lists the exit codes, which every tool
/// that answers `help` at its root has.
pub(crate) const TOPIC: &str = "exit-codes";

/// The summary that `<tool> help` prints beside [`TOPIC`].
pub(crate) const ABOUT: &str = "What each exit code means";

/// crumbkit's own exit codes and their meanings, which `<tool> help
/// exit-codes` lists before the tool's codes (ADR-0002). It leaves out 2
/// when the guide gives a refused command line another code. The meaning
/// of 1 names the rows that report a problem, because
/// [`exit::problem_rows`](crate::exit::problem_rows) returns 1.
const CRUMBKIT: [(u8, &str); 3] = [
    (0, "The command did what it was asked."),
    (
        1,
        "The command failed, or a row of its result reports a problem. A failure names its code and the next step.",
    ),
    (
        2,
        "The command line was refused. The refusal names the help to read.",
    ),
];

/// An exit code that the tool declares, with what it means, such as 124 for
/// a download that ran out of time.
///
/// A tool declares its codes in its [`Guide`], which `<tool> help
/// exit-codes` and `commands list` read:
///
/// ```
/// use crumbkit::help::{Exit, Guide};
///
/// const GUIDE: Guide = Guide::new().exits(&[
///     Exit::new(124, "The download ran out of time.").commands(&[&["books", "import"]]),
/// ]);
///
/// assert_eq!(GUIDE.get_exits()[0].code(), 124);
/// ```
///
/// An exit can list the failure codes that end a run with it, the tool's
/// own and crumbkit's. A run whose first failure has one of those codes
/// exits with the exit's number, in [`finish`](crate::finish), a batch, a
/// stream and a refusal alike, and any other failure exits with crumbkit's
/// default, which is 2 for a refused command line and 1 otherwise:
///
/// ```
/// use crumbkit::Code;
/// use crumbkit::help::{Exit, Guide};
///
/// const TIMED_OUT: Code = Code::new("shelf", "download", "timed_out");
///
/// const GUIDE: Guide = Guide::new().exits(&[
///     Exit::new(124, "The download ran out of time.").failures(&[&TIMED_OUT]),
/// ]);
///
/// assert_eq!(GUIDE.get_exits()[0].get_failures(), [&TIMED_OUT]);
/// ```
///
/// 0 means success, 1 and 2 keep crumbkit's meanings, and a shell reports
/// 126 and above for its own reasons, so a tool's codes run from 3 to 125
/// (ADR-0002). A tool that gives 1 or 2 a second meaning states its reason
/// with crumbkit-test's `Rule::SharedExit`. crumbkit-test's
/// `check_exit_codes` checks the declarations, and crumbkit never fails on
/// them at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    /// The exit code.
    code: u8,
    /// What the code means, as one sentence.
    meaning: &'static str,
    /// The paths of the commands that return the code, each covering the
    /// commands below it. An empty list covers every command.
    commands: &'static [&'static [&'static str]],
    /// The failure codes that end a run with the code.
    failures: &'static [&'static Code],
}

impl Exit {
    /// Returns the exit code `code`, which means `meaning`, a sentence that
    /// carries its own full stop. Every command of the tool can return it
    /// until [`Exit::commands`] names the ones that do.
    #[must_use]
    pub const fn new(code: u8, meaning: &'static str) -> Self {
        Self {
            code,
            meaning,
            commands: &[],
            failures: &[],
        }
    }

    /// Sets the commands that return the code. Each path holds the real
    /// names of the subcommands from the root, as [`Leaf::new`](super::Leaf::new)
    /// names a command, and covers that command and every command below
    /// it.
    #[must_use]
    pub const fn commands(mut self, paths: &'static [&'static [&'static str]]) -> Self {
        self.commands = paths;
        self
    }

    /// Sets the failure codes that end a run with the code. A run whose
    /// first failure has one of `codes` exits with this code in place of
    /// crumbkit's default (ADR-0037). A code may be one of crumbkit's, such
    /// as `crumbkit::codes::CLI_USAGE`, or the tool's rename of it from
    /// [`Guide::codes`](super::Guide::codes), and either covers both. A code that crumbkit learns only at run time, from
    /// `Code::runtime`, cannot be listed, so its failure exits with 1.
    #[must_use]
    pub const fn failures(mut self, codes: &'static [&'static Code]) -> Self {
        self.failures = codes;
        self
    }

    /// Returns the failure codes that [`Exit::failures`] set, or an empty
    /// list when no failure ends a run with the code.
    #[must_use]
    pub const fn get_failures(&self) -> &'static [&'static Code] {
        self.failures
    }

    /// Returns the exit code.
    #[must_use]
    pub const fn code(&self) -> u8 {
        self.code
    }

    /// Returns what the code means.
    #[must_use]
    pub const fn meaning(&self) -> &'static str {
        self.meaning
    }

    /// Returns the paths that [`Exit::commands`] set, or an empty list when
    /// every command returns the code. The name has clap's `get_` prefix,
    /// because `commands` is the builder method.
    #[must_use]
    pub const fn get_commands(&self) -> &'static [&'static [&'static str]] {
        self.commands
    }

    /// Returns whether the command at `path`, the real names of its
    /// subcommands from the root, can return the code: every command can
    /// when the code names no commands, and otherwise a command at or below
    /// one of its paths can. crumbkit-usage reads it to give each exit code
    /// in its usage reports its meaning.
    #[must_use]
    pub fn covers(&self, path: &[&str]) -> bool {
        self.commands.is_empty() || self.commands.iter().any(|scope| path.starts_with(scope))
    }
}

/// Returns the text that `<tool> help exit-codes` prints, where `tool` is
/// the tool's name: crumbkit's codes that `guide` keeps, then each code of
/// `guide` in order,
/// each with its meaning and, for a code that only some commands return,
/// those commands, a line for each leaf that passes on a program's exit
/// code, and a closing line about the codes that a shell reports.
pub(crate) fn text(tool: &str, guide: &Guide) -> String {
    let exits = guide.get_exits();
    let width = exits
        .iter()
        .map(|exit| digits(exit.code))
        .chain(std::iter::once(1))
        .max()
        .unwrap_or(1);
    let mut text = String::from("Exit codes\n");
    // Writing to a `String` cannot fail.
    for (code, meaning) in CRUMBKIT {
        if code == 2 && !refuses_with_2(guide) {
            continue;
        }
        let _ = writeln!(text, "  {code:<width$}  {meaning}");
    }
    for exit in exits {
        let _ = write!(text, "  {:<width$}  {}", exit.code, exit.meaning);
        for (index, path) in exit.commands.iter().enumerate() {
            let lead = if index == 0 { " Commands: " } else { ", " };
            let _ = write!(text, "{lead}`{tool}");
            for name in *path {
                let _ = write!(text, " {name}");
            }
            text.push('`');
        }
        if !exit.commands.is_empty() {
            text.push('.');
        }
        text.push('\n');
    }
    let passing: Vec<_> = guide
        .get_leaves()
        .iter()
        .filter_map(|leaf| Some((leaf.path().join(" "), leaf.get_passes_on_exit()?)))
        .collect();
    if !passing.is_empty() {
        text.push('\n');
    }
    for (path, program) in passing {
        let _ = writeln!(
            text,
            "`{tool} {path}` passes on the exit code of {program}."
        );
    }
    let _ = writeln!(
        text,
        "\nA shell reports 126 and above for its own reasons, such as 127 when it cannot find {tool} and 130 when Ctrl-C ends it."
    );
    text
}

/// Returns the note that `commands list` prints after its table, which
/// names the exit codes and the topic that describes them, such as
/// ``Exit codes: 0, 1 and 2 are crumbkit's, and shelf adds 124. `shelf
/// help exit-codes` describes them.``
pub(crate) fn note(tool: &str, guide: &Guide) -> String {
    let mut note = String::from(if refuses_with_2(guide) {
        "Exit codes: 0, 1 and 2 are crumbkit's"
    } else {
        "Exit codes: 0 and 1 are crumbkit's"
    });
    let exits = guide.get_exits();
    for (index, exit) in exits.iter().enumerate() {
        let lead = match index {
            0 => format!(", and {tool} adds "),
            _ if index + 1 == exits.len() => " and ".to_owned(),
            _ => ", ".to_owned(),
        };
        let _ = write!(note, "{lead}{}", exit.code);
    }
    let _ = write!(note, ". `{tool} help {TOPIC}` describes them.");
    note
}

/// Returns whether a refused command line exits with crumbkit's 2 under
/// `guide`, which lists no other code for it.
fn refuses_with_2(guide: &Guide) -> bool {
    guide.failure_exit(&[], &crate::codes::CLI_USAGE) == 2
}

/// Returns the number of decimal digits of `code`.
const fn digits(code: u8) -> usize {
    if code >= 100 {
        3
    } else if code >= 10 {
        2
    } else {
        1
    }
}
