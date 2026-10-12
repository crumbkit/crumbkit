//! Exit codes that a tool declares in its guide, which `<tool> help
//! exit-codes` and `commands list` describe.

use std::fmt::Write as _;

use super::Guide;

#[cfg(test)]
mod tests;

/// The name of the help topic that lists the exit codes, which every tool
/// that answers `help` at its root has.
pub(crate) const TOPIC: &str = "exit-codes";

/// The summary that `<tool> help` prints beside [`TOPIC`].
pub(crate) const ABOUT: &str = "What each exit code means";

/// crumbkit's own exit codes and their meanings, which `<tool> help
/// exit-codes` lists before the tool's codes (ADR-0002).
const CRUMBKIT: [(u8, &str); 3] = [
    (0, "The command did what it was asked."),
    (
        1,
        "The command failed. The failure names its code and the next step.",
    ),
    (
        2,
        "The command line was refused. The refusal names the help to read.",
    ),
];

/// An exit code that the tool returns besides crumbkit's 0, 1 and 2, with
/// what it means, such as 124 for a download that ran out of time.
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
/// crumbkit owns 0, 1 and 2, and a shell reports 126 and above for its own
/// reasons, so a tool's codes run from 3 to 125 (ADR-0002). crumbkit-test's
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
    /// one of its paths can.
    pub(crate) fn covers(&self, path: &[&str]) -> bool {
        self.commands.is_empty() || self.commands.iter().any(|scope| path.starts_with(scope))
    }
}

/// Returns the text that `<tool> help exit-codes` prints, where `tool` is
/// the tool's name: crumbkit's codes, then each code of `guide` in order,
/// each with its meaning and, for a code that only some commands return,
/// those commands, and a closing line about the codes that a shell reports.
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
    let mut note = String::from("Exit codes: 0, 1 and 2 are crumbkit's");
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
