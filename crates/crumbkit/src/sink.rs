//! The writers that a tool's output goes to, and the one place that maps
//! what each piece of output is to a stream (ADR-0128).

use std::io::{self, BufRead, Empty, IsTerminal as _, Stderr, StdinLock, StdoutLock, Write};

use self::role::{Deliver, Role};

pub(crate) mod role;
#[cfg(test)]
mod tests;

/// The destination of a tool's output, and the source of a caller's answer
/// to a question: a writer for standard output, a writer for standard
/// error and a reader for standard input.
///
/// crumbkit names what each piece of output is, and the sink chooses the
/// stream, the styling and the width from that. The run's result goes to
/// the output writer: the rows or the document of a command, help, the
/// version, a topic, a completion answer, and a failure document in JSON
/// form. Every message about the run goes to the error writer: a refusal,
/// the text form of a failure, a note, the dry-run line, a question, a
/// picker's echo, a debug build's warning and the report of a failed write.
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
    /// Receives each piece of output whose role is the result.
    out: O,
    /// Receives each piece of output whose role is a message.
    err: E,
    /// Holds the caller's answers to questions.
    input: I,
    /// Whether a person can answer a question: standard input and standard
    /// error are both terminals, and no coding agent runs the tool.
    interactive: bool,
    /// Whether the result is written with clap's styles, as a terminal
    /// shows them.
    styled: bool,
    /// The number of columns that a text table in the result fits, or
    /// `None` when it prints unfitted (ADR-0121).
    width: Option<usize>,
    /// The bytes of the result that the output writer took.
    result_bytes: u64,
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
            width: None,
            result_bytes: 0,
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
            width: None,
            result_bytes: 0,
        }
    }
}

impl<O, E, I> Sink<O, E, I> {
    /// Sets whether the result wants clap's styles, such as bold headings
    /// and literals, in help. With `styled`,
    /// [`parse_to`](crate::parse_to) writes help to the output writer with
    /// the ANSI escape codes that clap's `Error::print` writes at a
    /// terminal, and otherwise it writes plain text. The version and a
    /// topic carry no styles, so they print as plain text either way.
    ///
    /// A tool that owns its output streams decides it, for example from
    /// whether its writer is a terminal, and honours `NO_COLOR` itself:
    /// crumbkit writes the styles whenever the sink asks for them. clap
    /// renders styles only with its `color` feature, and crumbkit writes
    /// them only with its own `color` feature, which turns clap's on.
    /// Without it, a styled sink receives plain text. Every message prints
    /// as plain text on the error writer whatever this says.
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

    /// Reports whether the result wants clap's styles in help, which
    /// [`Sink::styled`] sets. A new sink asks for none.
    #[must_use]
    pub const fn is_styled(&self) -> bool {
        self.styled
    }

    /// Returns the sink with `width`, the number of columns that a text
    /// table in the result fits, as `finish` reads it from the terminal
    /// that standard output is (ADR-0121).
    pub(crate) const fn fitted(mut self, width: Option<usize>) -> Self {
        self.width = width;
        self
    }

    /// Returns the number of bytes of the result that the output writer
    /// took, which `finish` reports to the observer (ADR-0057).
    pub(crate) const fn result_bytes(&self) -> u64 {
        self.result_bytes
    }

    /// Reports whether a person can answer a question through the sink.
    pub(crate) const fn is_interactive(&self) -> bool {
        self.interactive
    }

    /// Returns a sink over the writers of this one, which asks for the same
    /// styles and width and reads no input.
    pub(crate) const fn by_ref(&mut self) -> Sink<&mut O, &mut E> {
        Sink {
            out: &mut self.out,
            err: &mut self.err,
            input: io::empty(),
            interactive: false,
            styled: self.styled,
            width: self.width,
            result_bytes: 0,
        }
    }

    /// Returns a sink that writes the result to `buffer` and every message
    /// to this sink's error writer, as a stream collects a refusal's
    /// document into its line before it sends the line.
    pub(crate) const fn buffering_result<'a>(
        &'a mut self,
        buffer: &'a mut Vec<u8>,
    ) -> Sink<&'a mut Vec<u8>, &'a mut E> {
        Sink {
            out: buffer,
            err: &mut self.err,
            input: io::empty(),
            interactive: false,
            styled: self.styled,
            width: self.width,
            result_bytes: 0,
        }
    }
}

/// The stream that a piece of output goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Writer {
    /// The output writer, which is standard output in `Sink::standard`.
    Output,
    /// The error writer, which is standard error in `Sink::standard`.
    Error,
}

/// Returns the stream that output with `role` goes to: the result to the
/// output writer, and every message to the error writer (ADR-0049,
/// OUTPUT-1). This is the one place that maps a role to a stream.
const fn writer_of(role: Role) -> Writer {
    match role {
        Role::Result => Writer::Output,
        Role::Message(_) => Writer::Error,
    }
}

impl<O: Write, E, I> Sink<O, E, I> {
    /// Writes `bytes` as a piece of the result to the output writer,
    /// counts the bytes that the writer took, and flushes it. A sink whose
    /// error writer cannot be written, as the sink of a completion answer
    /// may be, delivers its result here.
    pub(crate) fn deliver_result(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut counting = Counting {
            inner: &mut self.out,
            bytes: &mut self.result_bytes,
        };
        counting.write_all(bytes)?;
        self.out.flush()
    }
}

impl<O: Write, E: Write, I> Sink<O, E, I> {
    /// Returns the sink's input and the writer of `role`, which a question
    /// that a picker asks writes to while it reads the answer.
    pub(crate) fn dialogue(&mut self, role: Role) -> (&mut I, &mut dyn Write) {
        let writer: &mut dyn Write = match writer_of(role) {
            Writer::Output => &mut self.out,
            Writer::Error => &mut self.err,
        };
        (&mut self.input, writer)
    }
}

impl<O: Write, E: Write, I> Deliver for Sink<O, E, I> {
    fn deliver(&mut self, role: Role, bytes: &[u8]) -> io::Result<()> {
        match writer_of(role) {
            Writer::Output => self.deliver_result(bytes),
            Writer::Error => self.err.write_all(bytes).and_then(|()| self.err.flush()),
        }
    }

    fn styles(&self, role: Role) -> bool {
        match role {
            Role::Result => self.styled,
            // A message prints plain text. A record that colours a kind of
            // message changes this arm and no printing site.
            Role::Message(_) => false,
        }
    }

    fn width(&self, role: Role) -> Option<usize> {
        match role {
            Role::Result => self.width,
            Role::Message(_) => None,
        }
    }
}

/// A writer that passes every write to `inner` and adds the bytes that
/// `inner` took to `bytes`.
struct Counting<'a, W> {
    /// The writer that receives the bytes.
    inner: &'a mut W,
    /// The count that the bytes are added to.
    bytes: &'a mut u64,
}

impl<W: Write> Write for Counting<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        *self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
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

/// Reports whether the process's standard output is a terminal, which
/// [`try_parse_from`](crate::try_parse_from) reads to decide whether help
/// and the version are styled.
pub(crate) fn stdout_is_terminal() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the sink owns standard output, and help needs to know whether it is a terminal"
    )]
    let out = io::stdout();
    out.is_terminal()
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

/// Returns the number of columns of the terminal that the process's standard
/// output is, which `finish` fits a text table to (ADR-0121), or `None` when
/// standard output is not a terminal or the terminal reports a width of 0.
pub(crate) fn stdout_width() -> Option<usize> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the sink owns standard output, and a text table needs the width of its terminal"
    )]
    let out = io::stdout();
    if !out.is_terminal() {
        return None;
    }
    terminal_width(&out)
}

/// Returns the number of columns that the terminal `out` reports, or `None`
/// when it reports 0, as a pseudo-terminal whose size was never set does.
#[cfg(unix)]
fn terminal_width(out: &io::Stdout) -> Option<usize> {
    let size = rustix::termios::tcgetwinsize(out).ok()?;
    (size.ws_col > 0).then(|| usize::from(size.ws_col))
}

/// Returns `None`, because crumbkit reads no terminal width outside Unix.
///
/// RUST-10: Windows reports a console's width only through the console
/// API, which needs `unsafe` code or a new crate such as windows-sys
/// (ADR-0003), so a text table on Windows prints unfitted. A call to
/// `GetConsoleScreenBufferInfo` replaces this when a tool needs fitted
/// tables there.
#[cfg(not(unix))]
const fn terminal_width(_out: &io::Stdout) -> Option<usize> {
    None
}
