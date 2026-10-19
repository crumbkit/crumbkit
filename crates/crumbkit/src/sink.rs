//! The writers that a tool's result, its failure and its error lines go to.

use std::io::{self, BufRead, Empty, IsTerminal as _, Stderr, StdinLock, StdoutLock, Write};

/// The destination of a tool's output, and the source of a caller's answer
/// to a question: a writer for standard output, a writer for standard
/// error and a reader for standard input.
///
/// [`finish`](crate::finish) writes to the process's standard streams. Tests
/// build a sink over in-memory writers and pass it to
/// [`finish_to`](crate::finish_to), so they read what a tool would print
/// without running it.
///
/// ```
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let sink = crumbkit::Sink::new(&mut out, &mut err);
/// ```
///
/// A sink from [`Sink::new`] reads no input and is not interactive, so
/// [`Output::confirm_to`](crate::Output::confirm_to) never asks a question
/// through it. A test of a question builds the sink with
/// [`Sink::with_input`].
///
/// A sink also says whether its output writer wants styles, which
/// [`Sink::styled`] sets and [`parse_to`](crate::parse_to) reads when it
/// prints help or the version. A sink asks for no styles until
/// `Sink::styled` asks for them, so a test or a pipe receives plain text.
#[derive(Debug)]
pub struct Sink<O, E, I = Empty> {
    /// Receives the result, and the failure document with `--json`.
    pub(crate) out: O,
    /// Receives the text form of a failure and other error lines.
    pub(crate) err: E,
    /// Holds the caller's answers to questions.
    pub(crate) input: I,
    /// Whether a person can answer a question: standard input and standard
    /// error are both terminals, and no coding agent runs the tool.
    pub(crate) interactive: bool,
    /// Whether the output writer wants help with clap's styles, as a
    /// terminal does.
    styled: bool,
}

impl<O: Write, E: Write> Sink<O, E> {
    /// Returns a sink that writes the result to `out` and error lines to
    /// `err`, and reads no input.
    #[must_use]
    pub const fn new(out: O, err: E) -> Self {
        Self {
            out,
            err,
            input: io::empty(),
            interactive: false,
            styled: false,
        }
    }
}

impl<O: Write, E: Write, I: BufRead> Sink<O, E, I> {
    /// Returns a sink that writes the result to `out` and error lines to
    /// `err`, and reads answers from `input`. `interactive` says whether a
    /// person can answer a question, as when standard input and standard
    /// error are both terminals.
    ///
    /// ```
    /// let mut out = Vec::new();
    /// let mut err = Vec::new();
    /// let sink = crumbkit::Sink::with_input(&mut out, &mut err, &b"y\n"[..], true);
    /// ```
    #[must_use]
    pub const fn with_input(out: O, err: E, input: I, interactive: bool) -> Self {
        Self {
            out,
            err,
            input,
            interactive,
            styled: false,
        }
    }
}

impl<O, E, I> Sink<O, E, I> {
    /// Sets whether the output writer wants help with clap's styles, such
    /// as bold headings and literals. With `styled`,
    /// [`parse_to`](crate::parse_to) writes help to the output writer with
    /// the ANSI escape codes that clap's `Error::print` writes at a
    /// terminal, and otherwise it writes plain text. The version, a topic
    /// and the topic list carry no styles, so they print as plain text
    /// either way.
    ///
    /// A tool that owns its output streams decides it, for example from
    /// whether its writer is a terminal, and honours `NO_COLOR` itself:
    /// crumbkit writes the styles whenever the sink asks for them. clap
    /// renders styles only with its `color` feature, and crumbkit writes
    /// them only with its own `color` feature, which turns clap's on.
    /// Without it, a styled sink receives plain text.
    ///
    /// [`parse`](crate::parse) and [`parse_from`](crate::parse_from) build
    /// a sink over the process's standard streams that asks for styles when
    /// standard output is a terminal and the guide keeps terminal detection
    /// on.
    ///
    /// ```
    /// let sink = crumbkit::Sink::new(Vec::new(), Vec::new()).styled(true);
    /// assert!(sink.is_styled());
    /// ```
    #[must_use]
    pub const fn styled(mut self, styled: bool) -> Self {
        self.styled = styled;
        self
    }

    /// Reports whether the output writer wants help with clap's styles,
    /// which [`Sink::styled`] sets. A new sink asks for none.
    #[must_use]
    pub const fn is_styled(&self) -> bool {
        self.styled
    }
}

impl Sink<StdoutLock<'static>, Stderr> {
    /// Returns a sink over the process's standard output and standard error.
    /// This is the only place in crumbkit that opens the standard streams
    /// (RUST-4). A crate that prints a response without `finish`, such as
    /// crumbkit-child, prints through it, so it opens no stream itself.
    ///
    /// The sink asks for no styles, so the caller decides with
    /// [`Sink::styled`], as [`parse_from`](crate::parse_from) does from
    /// terminal detection.
    #[must_use]
    pub fn standard() -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "the sink owns standard output and locks it once for the whole result"
        )]
        let out = io::stdout().lock();
        #[expect(
            clippy::disallowed_methods,
            reason = "the sink owns standard error, where crumbkit prints a failure and reports a failed write or a bad flag"
        )]
        let err = io::stderr();
        Self::new(out, err)
    }
}

impl Sink<StdoutLock<'static>, Stderr, StdinLock<'static>> {
    /// Returns a sink over the process's standard output, standard error and
    /// standard input, which is interactive when standard input and standard
    /// error are both terminals and no coding agent runs the tool (ADR-0043).
    /// Only a question reads standard input, so only
    /// [`Output::confirm`](crate::Output::confirm) and a picker lock it, and
    /// a run that asks nothing leaves it unlocked and detects no agent.
    pub(crate) fn standard_with_input() -> Self {
        let Sink { out, err, .. } = Sink::standard();
        #[expect(
            clippy::disallowed_methods,
            reason = "the sink owns standard input, where a person answers a question"
        )]
        let input = io::stdin().lock();
        let terminals = input.is_terminal() && err.is_terminal();
        let interactive =
            crate::agent::person_can_answer(terminals, || crate::agent::detect().is_some());
        Self::with_input(out, err, input, interactive)
    }
}

/// Reports whether the process's standard error is a terminal, which
/// [`Output::progress_allowed`](crate::Output::progress_allowed) reads.
pub(crate) fn stderr_is_terminal() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the sink owns standard error, and progress needs to know whether it is a terminal"
    )]
    let err = io::stderr();
    err.is_terminal()
}
