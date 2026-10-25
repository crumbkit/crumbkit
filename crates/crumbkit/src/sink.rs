//! The writers that a tool's output goes to, and the one place that maps
//! what each piece of output is to a stream (ADR-0128).

use std::io::{self, BufRead, Empty, IsTerminal as _, Stderr, StdinLock, StdoutLock, Write};

pub(crate) use self::colour::stdout_styles;
#[cfg(test)]
use self::colour::{
    ColorVariables, clicolor_off, color_forced, color_off, term_dumb, wants_styles,
};
use self::role::{Deliver, Message, Role};

mod colour;
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
///
/// A sink says separately whether its error writer wants styles, which
/// [`Sink::styled_errors`] sets. A refusal and the group page printed for
/// it carry clap's styles on the error writer only then, and every other
/// message prints as plain text. A sink from [`Sink::new`] or
/// [`Sink::with_input`] wants no error styles (ADR-0131).
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
    /// Whether a refusal and its group page are written to the error
    /// writer with clap's styles (ADR-0131).
    styled_errors: bool,
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
            styled_errors: false,
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
            styled_errors: false,
            width: None,
            result_bytes: 0,
        }
    }
}

impl<O, E, I> Sink<O, E, I> {
    /// Sets whether the result wants styles: clap's styles in help, such as
    /// bold headings and literals, and the styles of a text table's cells.
    /// With `styled`,
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
    /// Without it, a styled sink receives plain text. This setting covers
    /// the result alone. [`Sink::styled_errors`] decides the styles of a
    /// refusal on the error writer, and every other message prints as
    /// plain text (ADR-0131).
    ///
    /// The same flag makes [`finish_to`](crate::finish_to) write the
    /// styles that [`Row::cell_style`](crate::Row::cell_style) gives the
    /// cells of a text table. crumbkit writes those escape codes itself,
    /// with or without the `color` feature (ADR-0150).
    ///
    /// [`parse`](crate::parse) and [`parse_from`](crate::parse_from) build
    /// a sink over the process's standard streams that asks for styles when
    /// standard output is a terminal and the guide keeps terminal detection
    /// on. [`finish`](crate::finish) asks for cell styles under the rule
    /// that [`Sink::styled_errors`] describes for `parse`, applied to
    /// standard output.
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

    /// Reports whether the result wants styles in help and in a text
    /// table's cells, which [`Sink::styled`] sets. A new sink asks for none.
    #[must_use]
    pub const fn is_styled(&self) -> bool {
        self.styled
    }

    /// Sets whether the error writer wants clap's styles for a refusal and
    /// for the group page printed in its place. With `styled`, a refusal's
    /// text carries clap's styles: the `error:` label, and the values and
    /// literals that clap styles in its own error, such as a refused value
    /// and the flag it was given to. A refused command line that stops at
    /// a group prints the group's page with clap's styles. Stripping the
    /// escape codes leaves the plain bytes. Every other message, such as
    /// the text form of a failure or a note, prints as plain text whatever
    /// this says (ADR-0131).
    ///
    /// The setter checks no terminal, as [`Sink::styled`] checks none. A
    /// tool that owns its error stream decides it, for example from whether
    /// standard error is a terminal, and honours `NO_COLOR` itself:
    /// crumbkit writes the styles whenever the sink asks for them. clap
    /// renders styles only with its `color` feature, and crumbkit writes
    /// them only with its own `color` feature, which turns clap's on.
    /// Without it, a refusal prints as plain text.
    ///
    /// [`parse`](crate::parse), [`parse_from`](crate::parse_from) and
    /// [`Refused::print`](crate::Refused::print) build a sink over the
    /// process's standard streams that asks for error styles when
    /// `CLICOLOR_FORCE` is set, or when standard error is a terminal whose
    /// `TERM` is not `dumb` and `CLICOLOR` is not `0`. It never asks for
    /// them under `NO_COLOR`, with the command's `ColorChoice` set to
    /// `Never` or with the guide's terminal detection off (ADR-0133).
    ///
    /// ```
    /// let sink = crumbkit::Sink::new(Vec::new(), Vec::new()).styled_errors(true);
    /// assert!(sink.is_styled_errors());
    /// assert!(!sink.is_styled());
    /// ```
    #[must_use]
    pub const fn styled_errors(mut self, styled: bool) -> Self {
        self.styled_errors = styled;
        self
    }

    /// Reports whether the error writer wants clap's styles for a refusal
    /// and its group page, which [`Sink::styled_errors`] sets. A new sink
    /// asks for none.
    #[must_use]
    pub const fn is_styled_errors(&self) -> bool {
        self.styled_errors
    }

    /// Sets `width`, the number of columns that a text table in the result
    /// fits. With `Some(width)`, [`finish_to`](crate::finish_to) narrows a
    /// text table wider than `width` and cuts its cells, except in the
    /// columns that [`Row::UNCUT_COLUMNS`](crate::Row::UNCUT_COLUMNS)
    /// names, and the output budget still counts the unfitted table. With
    /// `None`, the table prints unfitted. A width narrower than the headers
    /// and the uncut columns together, such as `Some(0)`, leaves the table
    /// unfitted, as a terminal that narrow does. JSON, prose, bytes, a
    /// row's own text line, a stream, help and every message print the
    /// same bytes whatever the width (ADR-0121, ADR-0134).
    ///
    /// The setter checks no terminal, as [`Sink::styled`] checks none. A
    /// tool that owns its output stream decides it, for example from the
    /// width of the terminal that standard output is. [`finish`](crate::finish)
    /// gives its sink that width when the guide keeps terminal detection
    /// on. A sink from [`Sink::new`] or [`Sink::with_input`] has no width.
    ///
    /// ```
    /// let sink = crumbkit::Sink::new(Vec::new(), Vec::new()).fitted(Some(80));
    /// assert_eq!(sink.fitted_width(), Some(80));
    /// assert!(!sink.is_styled());
    /// ```
    #[must_use]
    pub const fn fitted(mut self, width: Option<usize>) -> Self {
        self.width = width;
        self
    }

    /// Returns the number of columns that a text table in the result fits,
    /// which [`Sink::fitted`] sets, or `None` when the table prints
    /// unfitted. A new sink has no width.
    #[must_use]
    pub const fn fitted_width(&self) -> Option<usize> {
        self.width
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
            styled_errors: self.styled_errors,
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
            styled_errors: self.styled_errors,
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
            // A refusal and its group page carry clap's styles when the
            // error writer wants them (ADR-0131). Every other message prints
            // plain text.
            Role::Message(Message::Refusal | Message::RefusalHelp) => self.styled_errors,
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
