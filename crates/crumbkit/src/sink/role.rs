//! The role of each piece of output, which the site that prints the piece
//! names and the sink maps to a stream, a styling and a width (ADR-0128).

use std::io;

/// What one piece of output is. A printing site names the role of each
/// piece, and [`Sink`](crate::Sink) decides the stream, the styling and the
/// width from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// The run's answer: the rows or the document of a command, help, the
    /// version or a topic that the caller asked for, a completion answer,
    /// and a failure document in JSON form (ADR-0098, ADR-0100, ADR-0125).
    Result,
    /// A message that crumbkit prints about the run.
    Message(Message),
}

impl Role {
    /// Returns the role of a failure printed in the form that `json`
    /// chooses: the result for the JSON document, and the message `text`
    /// for the text form. The form is settled before the role, so a form
    /// that a tool fixed with `Refused::as_json` fixes the role.
    pub(crate) const fn of_failure(json: bool, text: Message) -> Self {
        if json {
            Self::Result
        } else {
            Self::Message(text)
        }
    }
}

/// The kinds of message that crumbkit prints about a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Message {
    /// The text form of a refused command line or view flag.
    Refusal,
    /// The help page of a group that a refused command line stopped at,
    /// which prints in place of the refusal's lines (ADR-0118).
    RefusalHelp,
    /// The text form of a failure: the run's, a picker source's, a batch
    /// entry's or a streamed one.
    Failure,
    /// A line that describes a result or a stream: a listing's footer or
    /// empty note, the cut line, the note of the rows that report a
    /// problem, the tool's note, the `Next:` line (ADR-0050) and the note
    /// that `--long` or `--full` adds nothing to the JSON (ADR-0149).
    Note,
    /// A `warning:` line that only a debug build prints, about a failure
    /// field or a recovery that crumbkit left out.
    DebugWarning,
    /// The line that marks a dry run.
    DryRun,
    /// The question of `Output::confirm` or of a picker, which a person
    /// answers.
    Prompt,
    /// The `Running:` line of a run whose arguments a picker filled in.
    Echo,
    /// The report that the result could not be written.
    FailedWrite,
}

/// Takes pieces of output with their roles. [`Sink`](crate::Sink)
/// implements it, and a function below a printing site takes a
/// `&mut dyn Deliver` in place of a pair of writers.
pub(crate) trait Deliver {
    /// Writes `bytes` as one piece of output with `role`, and flushes the
    /// writer that took them.
    ///
    /// # Errors
    ///
    /// Returns the error of the write or the flush.
    fn deliver(&mut self, role: Role, bytes: &[u8]) -> io::Result<()>;

    /// Reports whether output with `role` is written with clap's styles.
    fn styles(&self, role: Role) -> bool;

    /// Returns the number of columns that a text table with `role` fits,
    /// or `None` when the table prints unfitted.
    fn width(&self, role: Role) -> Option<usize>;

    /// Delivers the report that the result could not be written because of
    /// `error`. The process is already failing, and a failed report has
    /// nowhere else to go, so a failure of the report itself goes
    /// unreported.
    fn report_failed_write(&mut self, error: &io::Error) {
        let line = format!("error: cannot write to standard output: {error}\n");
        let _ = self.deliver(Role::Message(Message::FailedWrite), line.as_bytes());
    }
}

/// A sink for tests that records each piece of output with its role, in
/// order.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Recorded {
    /// Each piece that was delivered, with its role.
    pub(crate) pieces: Vec<(Role, Vec<u8>)>,
    /// Whether the result is written with clap's styles.
    pub(crate) styled: bool,
    /// The width that a result's text table fits.
    pub(crate) width: Option<usize>,
}

#[cfg(test)]
impl Recorded {
    /// Returns the role of each piece, in order.
    pub(crate) fn roles(&self) -> Vec<Role> {
        self.pieces.iter().map(|(role, _)| *role).collect()
    }

    /// Returns each piece as text with its role, in order.
    pub(crate) fn texts(&self) -> Vec<(Role, String)> {
        self.pieces
            .iter()
            .map(|(role, bytes)| (*role, String::from_utf8_lossy(bytes).into_owned()))
            .collect()
    }
}

#[cfg(test)]
impl Deliver for Recorded {
    fn deliver(&mut self, role: Role, bytes: &[u8]) -> io::Result<()> {
        self.pieces.push((role, bytes.to_vec()));
        Ok(())
    }

    fn styles(&self, role: Role) -> bool {
        role == Role::Result && self.styled
    }

    fn width(&self, role: Role) -> Option<usize> {
        self.width.filter(|_| role == Role::Result)
    }
}
