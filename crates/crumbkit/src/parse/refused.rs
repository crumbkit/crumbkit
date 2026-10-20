//! `Unparsed` and `Refused`, which `try_parse` returns when it does not
//! return a parsed command, so a tool can print a refusal itself.

use std::ffi::{OsStr, OsString};
use std::io::Write;

use clap::error::ErrorKind;

use super::walk::given;
use super::{Printed, Stop};
use crate::codes::CLI_USAGE;
use crate::refusal::Refusal;
use crate::{Code, Failure as _, Output, Recovery, Sink};

#[cfg(test)]
mod tests;

/// What [`try_parse`](crate::try_parse) returns when it returns no parsed
/// command.
///
/// A tool that prints every refusal as crumbkit does calls
/// [`Unparsed::print`], which returns the [`Stop`] that
/// [`parse`](crate::parse) would have returned.
#[derive(Debug)]
#[non_exhaustive]
pub enum Unparsed {
    /// crumbkit printed help, the version, a topic or the output of a
    /// picker that ended the run, and the run ends with this `Stop`, whose
    /// [`Stop::printed`] says which.
    Stopped(Stop),
    /// clap or crumbkit refused the command line, and nothing was printed.
    Refused(Refused),
}

impl Unparsed {
    /// Prints a refusal to the process's standard error, or to its
    /// standard output when the caller asked for JSON, as
    /// [`Refused::print`] does, and returns the `Stop` that ends the run.
    /// A `Stopped` run already printed, so it returns its `Stop` and
    /// prints nothing.
    #[must_use]
    pub fn print(self) -> Stop {
        match self {
            Self::Stopped(stop) => stop,
            Self::Refused(refused) => refused.print(),
        }
    }

    /// Prints a refusal to `sink` as [`Refused::print_to`] does, and
    /// returns the `Stop` that ends the run. A `Stopped` run already
    /// printed, so it returns its `Stop` and prints nothing.
    #[must_use]
    pub fn print_to<O: Write, E: Write, R>(self, sink: &mut Sink<O, E, R>) -> Stop {
        match self {
            Self::Stopped(stop) => stop,
            Self::Refused(refused) => refused.print_to(sink),
        }
    }
}

/// A command line that [`try_parse`](crate::try_parse) refused and has not
/// printed.
///
/// It holds the refusal that [`parse`](crate::parse) would print, the
/// words that the caller typed, the words that crumbkit gave clap, and the
/// output flags as far as the walk of the typed words found them. A tool
/// that resolves its own aliases after a refusal reads the words, and a
/// tool with its own failure document prints the refusal itself from the
/// message, the code and the recovery.
///
/// Printing consumes the refusal, so a run prints it at most once:
///
/// ```compile_fail
/// fn twice(refused: crumbkit::Refused) {
///     let _first = refused.print();
///     let _second = refused.print();
/// }
/// ```
///
/// The refusal sits in a `Box`, so a `Result` that holds a `Refused` stays
/// small on the path that parses.
#[derive(Debug)]
pub struct Refused {
    /// The refusal and what the run knew when crumbkit refused it.
    inner: Box<Inner>,
}

/// What a [`Refused`] holds.
#[derive(Debug)]
struct Inner {
    /// The refusal that `print` prints.
    refusal: Refusal,
    /// clap's error, or `None` when crumbkit refused a view flag after clap
    /// accepted the command line.
    error: Option<clap::Error>,
    /// The names of the subcommands from the root to the deepest command
    /// that the walk of the words reached.
    path: Vec<String>,
    /// The output flags as far as the walk of the typed words found them,
    /// with the command tree that crumbkit parsed against.
    output: Output,
    /// The words that the caller typed after the program name.
    typed: Vec<OsString>,
    /// The words after the program name that crumbkit gave clap.
    parsed: Vec<OsString>,
    /// The `crumbkit.parse` span, in which the refusal prints, so the span
    /// records its code and exit code (ADR-0030).
    #[cfg(feature = "tracing")]
    span: tracing::Span,
}

impl Refused {
    /// Returns the refusal `refusal` of a run whose output is `output`.
    /// `error` is clap's error, or `None` for a refused view flag, and
    /// `path` names the subcommands that the walk reached. `typed` and
    /// `parsed` are the words after the program name as the caller typed
    /// them and as crumbkit gave them to clap.
    pub(crate) fn new(
        refusal: Refusal,
        error: Option<clap::Error>,
        path: Vec<String>,
        output: Output,
        typed: Vec<OsString>,
        parsed: Vec<OsString>,
    ) -> Self {
        Self {
            inner: Box::new(Inner {
                refusal,
                error,
                path,
                output,
                typed,
                parsed,
                #[cfg(feature = "tracing")]
                span: tracing::Span::current(),
            }),
        }
    }

    /// Returns what the caller got wrong, on one line, as the refusal
    /// prints it, such as `unexpected argument '--srot' found; a similar
    /// argument exists: '--sort'`.
    #[must_use]
    pub fn message(&self) -> &str {
        self.inner.refusal.message()
    }

    /// Returns the code that the refusal prints: crumbkit's
    /// [`CLI_USAGE`](crate::codes::CLI_USAGE), or the tool's own code
    /// when the guide renames it.
    #[must_use]
    pub fn code(&self) -> &'static Code {
        self.inner.output.printing().renames.printed(&CLI_USAGE)
    }

    /// Returns the exit code that the run ends with once the refusal
    /// prints: 2, unless the guide lists another exit for a refused command
    /// line.
    #[must_use]
    pub fn number(&self) -> u8 {
        self.inner.output.printing().exit(&CLI_USAGE)
    }

    /// Returns what the caller does next, which the refusal prints after
    /// the message, such as running the `-h` of the deepest command.
    #[must_use]
    pub fn recovery(&self) -> Recovery {
        self.inner.refusal.recovery()
    }

    /// Returns the output flags as far as the walk of the typed words found
    /// them. Its [`Output::is_json`] says whether the caller asked for JSON.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.inner.output
    }

    /// Returns the words that the caller typed after the program name, as
    /// typed. `tool help nonsense` gives `help` and `nonsense`.
    #[must_use]
    pub fn typed(&self) -> &[OsString] {
        &self.inner.typed
    }

    /// Returns the words after the program name that crumbkit gave clap.
    /// They differ from [`Refused::typed`] when crumbkit turned the word
    /// `help` into `--help`, replaced a dashed form with the command's own
    /// name, or added a value that a picker picked. `tool help nonsense`
    /// gives `nonsense` and `--help`.
    #[must_use]
    pub fn parsed(&self) -> &[OsString] {
        &self.inner.parsed
    }

    /// Returns the real names of the subcommands from the root to the
    /// deepest command that the walk of the words reached, such as `books`
    /// and `show` for `tool books show --srot name`. The path is empty when
    /// the walk reached no subcommand. A tool that prints its own refusal
    /// names the deepest command's help from it, such as `tool books show
    /// --help`.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.inner.path
    }

    /// Returns the clap ids of the arguments that the words pass as flags,
    /// in the order typed, including the options that take values. A group
    /// of short flags such as `-qv` gives the id of each flag in it, and a
    /// name that no argument has gives nothing.
    ///
    /// The words are the ones that crumbkit gave clap, which
    /// [`Refused::parsed`] returns, so the word `help` that crumbkit turned
    /// into `--help` gives clap's help id, `help`. They are read as
    /// [`command_path`](crate::command_path) reads them, so the value of an
    /// option is never read as a flag.
    #[must_use]
    pub fn flags(&self) -> Vec<String> {
        given(self.inner.output.command_tree(), &self.inner.parsed, false).ids
    }

    /// Returns the last value that the words give the option whose clap id
    /// is `id`, or `None` when the words give it no value or `id` names no
    /// option. The option is the current command's own argument or a
    /// global argument of a command above it.
    ///
    /// The value is the word after the option's flag, or the text after
    /// the name in the flag's own word, as in `--format=json`, `-fjson` and
    /// `-f=json`. It comes back as one word, so a value that the option
    /// splits at its value delimiter comes back unsplit. The words are the
    /// ones that crumbkit gave clap, which [`Refused::parsed`] returns, and
    /// they are read as [`Refused::flags`] reads them. A tool that prints
    /// its own refusal reads a global option such as `--format` here to
    /// choose how to print it.
    #[must_use]
    pub fn value(&self, id: &str) -> Option<&OsStr> {
        let given = given(self.inner.output.command_tree(), &self.inner.parsed, false);
        given
            .values
            .iter()
            .rev()
            .find(|(arg, _)| arg.get_id().as_str() == id)
            .map(|&(_, value)| value)
    }

    /// Returns the word that the refusal names as wrong, or `None` when it
    /// names none, as for a missing argument. It is a subcommand that no
    /// command has, a value that its argument refused, an unexpected word
    /// or flag, or the view flag that crumbkit refused. clap names these
    /// through its error context, which needs clap's `error-context`
    /// feature (ADR-0003).
    #[must_use]
    pub fn culprit(&self) -> Option<&str> {
        let refusal = &self.inner.refusal;
        if let Some(word) = refusal
            .field("subcommand")
            .or_else(|| refusal.field("value"))
        {
            return Some(word);
        }
        let argument = refusal.field("argument")?;
        match self.inner.error.as_ref().map(clap::Error::kind) {
            None | Some(ErrorKind::UnknownArgument) => Some(argument),
            Some(ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand) => None,
            Some(_) => argument.starts_with('-').then_some(argument),
        }
    }

    /// Returns clap's error for the refused command line, or `None` for a
    /// refused view flag, which crumbkit refuses itself after clap accepted
    /// the line. A tool that builds its own sentence from the error's kind
    /// and context, such as the argument that conflicted or was repeated,
    /// reads `kind()` and `get(ContextKind)` here instead of parsing the
    /// line again. clap fills that context through its `error-context`
    /// feature, which crumbkit enables (ADR-0003).
    #[must_use]
    pub fn error(&self) -> Option<&clap::Error> {
        self.inner.error.as_ref()
    }

    /// Prints the refusal to the process's standard error, or to its
    /// standard output when the caller asked for JSON, as
    /// [`parse`](crate::parse) prints it, and returns the `Stop` that ends
    /// the run.
    #[must_use]
    pub fn print(self) -> Stop {
        self.print_to(&mut Sink::standard())
    }

    /// Prints the refusal to `sink` as [`parse_to`](crate::parse_to)
    /// prints it, and returns the `Stop` that ends the run.
    ///
    /// The run's observers hear of the refusal when it prints, and not
    /// when `try_parse` returns it, so a run that prints no refusal reports
    /// none.
    #[must_use]
    pub fn print_to<O: Write, E: Write, R>(self, sink: &mut Sink<O, E, R>) -> Stop {
        let Inner {
            refusal,
            error,
            path,
            mut output,
            #[cfg(feature = "tracing")]
            span,
            ..
        } = *self.inner;
        #[cfg(feature = "tracing")]
        let _entered = span.enter();
        let number = output.printing().exit(&CLI_USAGE);
        match &error {
            Some(error) => crate::events::refused(output.command_tree(), &path, error, number),
            None => crate::events::exited(number),
        }
        let code = output.print_parse_refusal(sink, &refusal);
        #[cfg(feature = "tracing")]
        crate::trace::spans::exited(&span, code);
        Stop::with_output(Printed::Refusal, code, output)
    }
}
