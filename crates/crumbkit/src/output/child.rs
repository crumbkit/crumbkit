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
        failure::print(
            sink,
            self.json,
            spelling,
            self.failure_form,
            self.json_layout,
            failure,
            self.command.as_mut(),
        );
    }

    /// Prints the refusal of a `--fields` name that a `T` lacks, or of an
    /// output flag that a `T` refuses, and returns exit code 2, as `finish`
    /// does, or returns `None` when `T` accepts the flags.
    pub(crate) fn refuse_fields<T: Rows, O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
    ) -> Option<ExitCode> {
        let refusal = fields::flags_refusal::<T>(self)?;
        Some(self.print_refusal(sink, spelling, &refusal))
    }

    /// Prints the refusal of `--fields` for a child that holds the terminal,
    /// whose own text has no columns, and returns exit code 2, or returns
    /// `None` when the caller passed no `--fields`.
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

    /// Prints `refusal`, records exit code 2 in the usage log, and returns
    /// it.
    fn print_refusal<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        refusal: &Refusal,
    ) -> ExitCode {
        // `print` returns exit code 2, the code that `ended` records and
        // returns.
        let _ = refusal.print(
            sink,
            self.json,
            spelling,
            self.failure_form,
            self.json_layout,
            self.command.as_mut(),
        );
        crate::usage::ended(2, None)
    }
}
