//! What a crate that prints a response without `finish` reads from the
//! output flags and prints through them, such as crumbkit-child, which
//! hands the terminal to another program.

use std::io::Write;
use std::process::ExitCode;

use super::{Run, failure, fields};
use crate::refusal::Refusal;
use crate::{Code, Document, Failure, Rows, Sink};

impl Run {
    /// Prints `failure` to `sink` as `finish` prints a failure, in the format
    /// that the flags choose, with a `Recovery::Run` rendered against the
    /// command that the run holds.
    ///
    /// It returns no exit code, for a command that passes on another code
    /// or counts the bytes it printed. That caller reports its code and
    /// count with [`events::ended`](crate::events::ended), and
    /// [`Run::failure_exit`] gives the failure's own exit code. A command
    /// that ends with this failure's own exit code and has no count calls
    /// [`Run::fail`].
    pub fn print_failure<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        failure: &dyn Failure,
    ) {
        let printing = self.printing();
        failure::print(
            sink,
            self.is_json(),
            self.spelling,
            printing,
            failure,
            &mut self.command,
        );
    }

    /// Prints the refusal of the view flags that a `T` in `document` cannot
    /// use, such as a `--fields` name that `T` lacks, and returns the
    /// refusal's exit code. It applies the rule that `finish` applies to a
    /// successful result. It returns `None` when `T` accepts the flags.
    ///
    /// A command that does slow work before it calls `finish` calls it
    /// first, so the caller learns of a mistyped flag before the work
    /// starts. The exit code is 2 unless the tool record lists another for
    /// `<tool>::cli::usage`, and it is reported to the observer.
    pub fn refuse_view<T: Rows, O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        document: Document<T>,
    ) -> Option<ExitCode> {
        let refusal = fields::flags_refusal(self, document)?;
        Some(self.print_refusal(sink, &refusal))
    }

    /// Prints the refusal of `--fields` for a command whose response has
    /// no columns, and returns the refusal's exit code, or returns `None`
    /// when the caller passed no `--fields`.
    ///
    /// The message is `--fields selects the columns of a table, and` followed
    /// by `reason`, such as `this command hands the terminal to another
    /// program`. The exit code is 2 unless the tool record lists another for
    /// `<tool>::cli::usage`, and it is reported to the observer.
    pub fn refuse_fields_for_text<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        reason: &str,
    ) -> Option<ExitCode> {
        self.view.names()?;
        let refusal = Refusal::flag_value(
            format!("--fields selects the columns of a table, and {reason}"),
            "run the command without --fields".to_owned(),
            "--fields",
            None,
        );
        Some(self.print_refusal(sink, &refusal))
    }

    /// Prints `refusal`, a command line or a view flag that `parse`
    /// refused, to `sink` as `parse` prints it, in the view's format and
    /// with what the tool record decides about failures, and returns the exit
    /// code that the tool record gives a refused command line.
    pub(crate) fn print_parse_refusal<O: Write, E: Write, R>(
        &mut self,
        sink: &mut Sink<O, E, R>,
        refusal: &Refusal,
    ) -> u8 {
        refusal.print(
            sink,
            self.is_json(),
            self.spelling,
            self.printing(),
            &mut self.command,
        )
    }

    /// Prints `refusal`, reports its exit code to the observer, and
    /// returns it. The exit code is 2 unless the tool record lists another for
    /// the usage code.
    fn print_refusal<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        refusal: &Refusal,
    ) -> ExitCode {
        let printing = self.printing();
        let exit = refusal.print(
            sink,
            self.is_json(),
            self.spelling,
            printing,
            &mut self.command,
        );
        crate::events::ended(exit, None)
    }

    /// Prints `failure` as [`Run::print_failure`] does, reports the
    /// failure's exit code to each observer with
    /// [`events::ended`](crate::events::ended), and returns it. A command
    /// that ends without [`finish`](crate::finish) returns this from `main`,
    /// so the run's end is reported however it ended. It reports the bytes
    /// of the result as `None`, unknown. A command that counted them calls
    /// [`Run::print_failure`] and [`events::ended`](crate::events::ended).
    #[must_use]
    pub fn fail<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        failure: &dyn Failure,
    ) -> ExitCode {
        self.print_failure(sink, failure);
        crate::events::ended(self.failure_exit(failure.code()), None)
    }

    /// Returns the exit code of a run whose first failure has `code`, as
    /// [`Tool::failure_exit`](crate::Tool::failure_exit) decides
    /// for the tool record that the run copied: the exit that
    /// lists the code, or crumbkit's default of 2 for a refused command
    /// line and 1 for every other failure.
    #[must_use]
    pub fn failure_exit(&self, code: Code<'_>) -> u8 {
        self.printing().exit(code)
    }

    /// Returns the exit code of the tool record's exit that lists `code`, or a
    /// crumbkit code that the tool record prints as `code`, and `None` when no
    /// single code lists it, as when only a range of codes lists it. A
    /// caller whose failure has a default other than crumbkit's reads it,
    /// and uses its own default for `None`.
    #[must_use]
    pub fn listed_exit(&self, code: Code<'_>) -> Option<u8> {
        self.printing().listed_exit(code)
    }
}
