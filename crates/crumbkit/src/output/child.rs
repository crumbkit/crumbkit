//! What [`crate::child`] reads from the output flags and prints through
//! them.

use std::io::Write;
use std::process::ExitCode;

use super::{Output, failure, fields};
use crate::refusal::Refusal;
use crate::{Failure, Rows, Sink, Spelling};

impl Output {
    /// Prints `failure` to `sink` as `finish` prints a failure, in the
    /// format that the flags choose, with a `Run` recovery rendered against
    /// the command that [`Output::set_command`] kept.
    pub(crate) fn print_failure<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        failure: &dyn Failure,
    ) {
        let printing = self.printing();
        failure::print(
            sink,
            self.json,
            spelling,
            printing,
            failure,
            self.command.as_mut(),
        );
    }

    /// Prints the refusal of a `--fields` name that a `T` lacks, or of a
    /// view flag that a `T` in a document with crumbkit's envelope, when
    /// `envelope` is set, cannot use, and returns the refusal's exit code,
    /// as `finish` does, or returns `None` when `T` accepts the flags.
    pub(crate) fn refuse_fields<T: Rows, O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        envelope: bool,
    ) -> Option<ExitCode> {
        let refusal = fields::flags_refusal::<T>(self, envelope)?;
        Some(self.print_refusal(sink, spelling, &refusal))
    }

    /// Prints the refusal of `--fields` for a child that holds the terminal,
    /// whose own text has no columns, and returns the refusal's exit code,
    /// or returns `None` when the caller passed no `--fields`.
    pub(crate) fn refuse_fields_for_child<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
    ) -> Option<ExitCode> {
        if self.fields.is_empty() {
            return None;
        }
        let refusal = Refusal::flag_value(
            "--fields selects the columns of a table, and this command hands the terminal to another program".to_owned(),
            "run the command without --fields".to_owned(),
            "--fields",
            None,
        );
        Some(self.print_refusal(sink, spelling, &refusal))
    }

    /// Prints `refusal`, reports its exit code to the observer, and
    /// returns it. The exit code is 2 unless the guide lists another for
    /// the usage code.
    fn print_refusal<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        refusal: &Refusal,
    ) -> ExitCode {
        let printing = self.printing();
        let exit = refusal.print(sink, self.json, spelling, printing, self.command.as_mut());
        crate::events::ended(exit, None)
    }

    /// Returns the exit code of a run whose first failure has `code`, as
    /// the guide that `set_command` copied decides.
    pub(crate) fn failure_exit(&self, code: &'static crate::Code) -> u8 {
        self.printing().exit(code)
    }
}
