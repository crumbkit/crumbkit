//! A next step that a listing's text output names with a sentence and the
//! commands that carry it out, and the other programs whose commands such a
//! step may name.

use std::borrow::Cow;

use clap::Command;

use crate::Call;
use crate::help::words::quote;
use crate::output::table::escape_controls;

#[cfg(test)]
mod tests;

/// Another program whose commands a [`Step`] names, such as `git`.
///
/// A step names a command of another program only through a `Program`,
/// which a tool declares as a constant so the check runs at build time, and
/// a step never holds a free command line. The declaration makes the tool's author decide that the
/// tool's guidance depends on that program, and it gives every step that
/// names the program the same spelling.
///
/// ```
/// use crumbkit::Program;
///
/// const GIT: Program = Program::new("git");
/// ```
///
/// A tool's own commands are named with a [`Call`], which crumbkit checks
/// against the tool's command tree, and never with a `Program`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Program {
    /// The word that runs the program, such as `git`.
    name: &'static str,
}

impl Program {
    /// Returns the program that the word `name` runs.
    ///
    /// # Panics
    ///
    /// Panics when `name` is not one word of ASCII letters, digits and the
    /// marks `-`, `_`, `.` and `+`, starting with a letter or a digit. A name
    /// therefore holds no arguments, no path and nothing that a shell would
    /// split or expand, and never reads as a flag. In a constant, the panic
    /// stops the build.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        let bytes = name.as_bytes();
        assert!(
            !bytes.is_empty() && bytes[0].is_ascii_alphanumeric(),
            "a program name is one word of ASCII letters, digits, `-`, `_`, `.` and `+`, starting with a letter or a digit"
        );
        let mut at = 1;
        while at < bytes.len() {
            let byte = bytes[at];
            assert!(
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+'),
                "a program name is one word of ASCII letters, digits, `-`, `_`, `.` and `+`, starting with a letter or a digit"
            );
            at += 1;
        }
        Self { name }
    }
}

/// A next step: a sentence that says what to do, and the commands that do it,
/// in order.
///
/// `crumbkit::finish` prints each step of a listing on standard error in text,
/// after the `Next:` line of [`Listing::next`](crate::Listing::next), as the
/// line `Next: {sentence}` followed by each command on its own line, indented
/// by two spaces. The JSON document leaves the steps out.
///
/// A command of the tool itself is a [`Call`], added with [`Step::call`]. A
/// command of another program is a [`Program`] with its arguments, added with
/// [`Step::run`].
///
/// ```
/// use crumbkit::{Call, Program, Step};
///
/// const GIT: Program = Program::new("git");
///
/// let step = Step::new("Commit the shelf, then list the books.")
///     .run(GIT, ["commit", "-m", "Add books"])
///     .call(Call::new(&["books", "list"]));
/// ```
///
/// prints, for a tool named `shelf`:
///
/// ```text
/// Next: Commit the shelf, then list the books.
///   git commit -m 'Add books'
///   shelf books list
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The sentence that says what to do, which carries its own full stop.
    sentence: String,
    /// The commands that carry out the step, in order.
    commands: Vec<StepCommand>,
}

/// One command of a [`Step`].
#[derive(Clone, Debug, PartialEq, Eq)]
enum StepCommand {
    /// A command of the tool itself, which renders against its command tree.
    Call(Call),
    /// A command of another program.
    Program {
        /// The program that runs the command.
        program: Program,
        /// The arguments, as the caller types them before a shell quotes them.
        args: Vec<String>,
    },
}

impl Step {
    /// Returns a step with the sentence `sentence` and no commands. The
    /// sentence prints unchanged, so it carries its own full stop. A step
    /// without commands prints its `Next:` line alone.
    #[must_use]
    pub fn new(sentence: impl Into<String>) -> Self {
        Self {
            sentence: sentence.into(),
            commands: Vec::new(),
        }
    }

    /// Adds the tool's own command `call` after the commands already added.
    ///
    /// `crumbkit::finish` renders it against the tool's command tree. When it
    /// does not resolve, because it names a missing or hidden command or
    /// argument, the whole step is left out, so a step never prints with one
    /// of its commands missing.
    #[must_use]
    pub fn call(mut self, call: Call) -> Self {
        self.commands.push(StepCommand::Call(call));
        self
    }

    /// Adds the command of `program` with the arguments `args` after the
    /// commands already added. It prints as the program's name followed by
    /// each argument, and an argument that a POSIX shell would split or
    /// expand prints between single quotes, as a [`Call`]'s values do.
    ///
    /// Each argument prints as a literal word. A placeholder such as
    /// `<VERSION>` prints quoted, because crumbkit cannot know the other
    /// program's usage. When an argument holds a control character or a
    /// Unicode format character, such as a line feed, an escape or a
    /// bidirectional override, the whole step is left out, as for a call
    /// that does not resolve, because such a character would reach the
    /// terminal unescaped.
    #[must_use]
    pub fn run<I, A>(mut self, program: Program, args: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<String>,
    {
        self.commands.push(StepCommand::Program {
            program,
            args: args.into_iter().map(Into::into).collect(),
        });
        self
    }

    /// Returns the lines of the step with their line feeds: the `Next:` line
    /// and each command indented by two spaces, with the tool's own calls
    /// rendered against `command`. It returns `None` when one of those calls
    /// does not resolve, or when an argument of another program holds a
    /// character that the text output escapes.
    pub(crate) fn lines(&self, command: &mut Command) -> Option<String> {
        let mut lines = format!("Next: {}\n", self.sentence);
        for step_command in &self.commands {
            let line = match step_command {
                StepCommand::Call(call) => crate::call::render(call, command).ok()?,
                StepCommand::Program { program, args } => program_line(*program, args)?,
            };
            lines.push_str("  ");
            lines.push_str(&line);
            lines.push('\n');
        }
        Some(lines)
    }
}

/// Returns the command line that runs `program` with `args`, or `None` when
/// an argument holds a character that `escape_controls` replaces.
fn program_line(program: Program, args: &[String]) -> Option<String> {
    let mut line = program.name.to_owned();
    for arg in args {
        if matches!(escape_controls(arg), Cow::Owned(_)) {
            return None;
        }
        line.push(' ');
        line.push_str(&quote(arg));
    }
    Some(line)
}
