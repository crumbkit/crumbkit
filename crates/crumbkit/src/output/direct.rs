//! What a crate that prints a response without `finish` reads from the
//! output flags and prints through them, such as crumbkit-child, which
//! hands the terminal to another program.

use std::io::Write;
use std::process::ExitCode;

use super::{Output, failure, fields};
use crate::refusal::Refusal;
use crate::{Code, Document, Failure, Rows, Sink, Spelling};

impl Output {
    /// Prints `failure` to `sink` as `finish` prints a failure, in the
    /// format that the flags choose and with the code that the guide
    /// prints for it, with a `Run` recovery rendered against the command
    /// that the output holds.
    ///
    /// It returns no exit code. The caller returns
    /// [`Output::failure_exit`] of the failure's code, or another code that
    /// the command passes on, and reports it with
    /// [`events::ended`](crate::events::ended).
    pub fn print_failure<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        failure: &dyn Failure,
    ) {
        let printing = self.printing();
        failure::print(
            sink,
            self.json,
            spelling.for_tool(self.tool),
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
    /// starts. The exit code is 2 unless the guide lists another for
    /// `<tool>::cli::usage`, and it is reported to the observer.
    pub fn refuse_view<T: Rows, O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        document: Document<T>,
    ) -> Option<ExitCode> {
        let refusal = fields::flags_refusal(self, document)?;
        Some(self.print_refusal(sink, spelling, &refusal))
    }

    /// Prints the refusal of `--fields` for a command whose response has
    /// no columns, and returns the refusal's exit code, or returns `None`
    /// when the caller passed no `--fields`.
    ///
    /// The message is `--fields selects the columns of a table, and`
    /// followed by `reason`, such as `this command hands the terminal to
    /// another program`. The exit code is 2 unless the guide lists another
    /// for `<tool>::cli::usage`, and it is reported to the observer.
    pub fn refuse_fields_for_text<O: Write, W: Write>(
        &mut self,
        sink: &mut Sink<O, W>,
        spelling: Spelling,
        reason: &str,
    ) -> Option<ExitCode> {
        if self.fields.is_empty() {
            return None;
        }
        let refusal = Refusal::flag_value(
            format!("--fields selects the columns of a table, and {reason}"),
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
        let spelling = spelling.for_tool(self.tool);
        let exit = refusal.print(sink, self.json, spelling, printing, &mut self.command);
        crate::events::ended(exit, None)
    }

    /// Returns the exit code of a run whose first failure has `code`, as
    /// [`Guide::failure_exit`](crate::help::Guide::failure_exit) decides
    /// for the guide that the output copied: the exit that
    /// lists the code, or crumbkit's default of 2 for a refused command
    /// line and 1 for every other failure.
    #[must_use]
    pub fn failure_exit(&self, code: &'static Code) -> u8 {
        self.printing().exit(code)
    }

    /// Returns the exit code of the guide's exit that lists `code`, or a
    /// crumbkit code that the guide prints as `code`, and `None` when no
    /// exit lists it. A caller whose failure has a default other than
    /// crumbkit's reads it, and uses its own default for `None`.
    #[must_use]
    pub fn listed_exit(&self, code: &'static Code) -> Option<u8> {
        self.printing().listed_exit(code)
    }
}
