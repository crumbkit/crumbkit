//! Exit codes that a tool declares in its guide, which `<tool> help
//! exit-codes` and `commands list` describe.

use std::fmt::Write as _;
use std::ops::RangeInclusive;

use super::Guide;
use crate::Code;

#[cfg(test)]
mod tests;

/// The name of the help topic that lists the exit codes, which every tool
/// that answers `help` at its root has.
pub(crate) const TOPIC: &str = "exit-codes";

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
/// a download that ran out of time, or a range of codes with one meaning,
/// such as the codes that a command passes on from a test suite it runs.
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
/// A command that passes on the exit code of another program declares the
/// range of codes it passes on with [`Exit::range`], and the guide's leaf
/// for it declares [`Leaf::passes_on_exit`](super::Leaf::passes_on_exit):
///
/// ```
/// use crumbkit::help::{Exit, Guide};
///
/// const GUIDE: Guide = Guide::new().exits(&[
///     Exit::range(3, 125, "The test suite failed with this code.").commands(&[&["suite", "run"]]),
///     Exit::new(130, "Ctrl-C ended the run."),
/// ]);
///
/// assert_eq!(GUIDE.get_exits()[0].codes(), 3..=125);
/// ```
///
/// 0 means success, 1 and 2 keep crumbkit's meanings, and a shell reports
/// 126 to 128 for its own reasons, such as 127 when it cannot find the
/// tool. A single code therefore runs from 3 to 125, or from 129 to 159
/// for a run that a signal ended, which a shell reports as 128 plus the
/// signal's number, such as 130 after Ctrl-C. A range runs from 3 to 255,
/// because its codes come from another program (ADR-0002, ADR-0103). A
/// tool that gives 1 or 2 a second meaning states its reason with
/// crumbkit-test's `Rule::SharedExit`, and a tool that declares another
/// single code from 126 states it with `Rule::ShellExitCode`.
/// crumbkit-test's `check_exit_codes` checks the declarations, and crumbkit
/// never fails on them at runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    /// The exit code, or the first code of a range.
    first: u8,
    /// The last code of a range, which equals `first` for a single code.
    last: u8,
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
        Self::range(code, code, meaning)
    }

    /// Returns the range of exit codes from `first` to `last`, both
    /// included, which all mean `meaning`, a sentence that carries its own
    /// full stop. A range declares the codes that a command passes on from
    /// another program, such as a child process or a test suite, whose
    /// codes the tool cannot name one by one. Every command of the tool
    /// can return them until [`Exit::commands`] names the ones that do.
    ///
    /// A range of one code is a single code. crumbkit-test's
    /// `check_exit_codes` fails on a range that starts below 3, ends
    /// before it starts, overlaps another declared code or lists failure
    /// codes.
    #[must_use]
    pub const fn range(first: u8, last: u8, meaning: &'static str) -> Self {
        Self {
            first,
            last,
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
    /// crumbkit's default (ADR-0037). A range names no single code to exit
    /// with, so the failures of a range never decide a run's exit, and
    /// crumbkit-test's `check_exit_codes` fails on a range that lists
    /// any. A code may be one of crumbkit's, such
    /// as `crumbkit::codes::CLI_USAGE`, or the tool's rename of it from
    /// [`Guide::codes`](super::Guide::codes), and either covers both.
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

    /// Returns the exit code, or the first code of a range.
    #[must_use]
    pub const fn code(&self) -> u8 {
        self.first
    }

    /// Returns the codes that the exit declares, from its first code to its
    /// last, both included. A single code returns a range of that one
    /// code, such as `124..=124`, and a range that ends before it starts
    /// returns an empty range.
    #[must_use]
    pub const fn codes(&self) -> RangeInclusive<u8> {
        RangeInclusive::new(self.first, self.last)
    }

    /// Returns whether the exit declares one code: a code from
    /// [`Exit::new`], or a range whose first and last codes are the same.
    pub(crate) const fn is_single(&self) -> bool {
        self.first == self.last
    }

    /// Returns the code as `help exit-codes` and `commands list` print
    /// it: the code, such as `124`, or the first and last codes of a range
    /// joined by a dash, such as `124-125`.
    fn label(&self) -> String {
        if self.is_single() {
            self.first.to_string()
        } else {
            format!("{}-{}", self.first, self.last)
        }
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
/// the tool's name: crumbkit's codes that `guide` keeps, then each code or
/// range of `guide` in order,
/// each with its meaning and, for a code that only some commands return,
/// those commands, a line for each leaf that passes on a program's exit
/// code, and a closing line about the codes that a shell reports.
pub(crate) fn text(tool: &str, guide: &Guide) -> String {
    let exits = guide.get_exits();
    let labels: Vec<String> = exits.iter().map(Exit::label).collect();
    let width = labels
        .iter()
        .map(String::len)
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
    for (exit, label) in exits.iter().zip(&labels) {
        let _ = write!(text, "  {label:<width$}  {}", exit.meaning);
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
/// ``Exit codes: 0, 1 and 2 apply to every command, and shelf adds 3 and
/// 124-125. `shelf help exit-codes` describes them.``
pub(crate) fn note(tool: &str, guide: &Guide) -> String {
    let mut note = String::from(if refuses_with_2(guide) {
        "Exit codes: 0, 1 and 2 apply to every command"
    } else {
        "Exit codes: 0 and 1 apply to every command"
    });
    let exits = guide.get_exits();
    for (index, exit) in exits.iter().enumerate() {
        let lead = match index {
            0 => format!(", and {tool} adds "),
            _ if index + 1 == exits.len() => " and ".to_owned(),
            _ => ", ".to_owned(),
        };
        let _ = write!(note, "{lead}{}", exit.label());
    }
    let _ = write!(note, ". `{tool} help {TOPIC}` describes them.");
    note
}

/// Returns whether a refused command line exits with crumbkit's 2 under
/// `guide`, which lists no other code for it.
fn refuses_with_2(guide: &Guide) -> bool {
    guide.failure_exit(&[], &crate::codes::CLI_USAGE) == 2
}
