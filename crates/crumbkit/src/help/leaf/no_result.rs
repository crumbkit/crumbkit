//! The declaration that a leaf's command prints no result on standard
//! output (ADR-0143).

use super::Leaf;

#[cfg(test)]
mod tests;

/// The line that the `-h` and `--help` of a leaf set with
/// [`Leaf::prints_no_result`] print after the effects line.
pub(super) const NO_RESULT_LINE: &str =
    "Output: no result on standard output, and --json prints a failure as JSON.";

impl Leaf {
    /// Declares that the command prints no result on standard output, such
    /// as a server that runs until the caller interrupts it and writes only
    /// log lines to standard error. The leaf then has no document and
    /// publishes no schema.
    ///
    /// The leaf offers none of `--fields`, `--long` and `--full`, so `parse`
    /// refuses each of them before the handler runs and the command's help
    /// leaves them out. `--json` stays, so a failure prints as the failure
    /// document. The command's `-h` and `--help` print `Output: no result
    /// on standard output, and --json prints a failure as JSON.` after the
    /// effects. crumbkit-test's `check_guide` accepts such a leaf whose
    /// effects change something without an opt-out, and fails a leaf that
    /// also declares a document.
    ///
    /// The command never succeeds. It ends with a failure, such as the
    /// interrupt that stopped the server, which it prints with
    /// [`Output::print_failure`](crate::Output::print_failure). It takes the
    /// exit code from [`Output::failure_exit`](crate::Output::failure_exit)
    /// and returns it through [`events::ended`](crate::events::ended), so
    /// each observer receives it:
    ///
    /// ```no_run
    /// use std::process::ExitCode;
    ///
    /// use crumbkit::help::{Guide, Leaf};
    /// use crumbkit::{Failure, Output, Sink};
    ///
    /// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["serve"]).prints_no_result()]);
    ///
    /// /// Ends a run of `serve` with `failure`, such as the interrupt that
    /// /// stopped the server.
    /// fn end(output: &mut Output, failure: &dyn Failure) -> ExitCode {
    ///     output.print_failure(&mut Sink::standard(), failure);
    ///     let exit = output.failure_exit(failure.code());
    ///     crumbkit::events::ended(exit, Some(0))
    /// }
    /// ```
    #[must_use]
    pub const fn prints_no_result(mut self) -> Self {
        self.no_result = true;
        self
    }

    /// Reports whether [`Leaf::prints_no_result`] declared that the command
    /// prints no result. The name has clap's `get_` prefix, because
    /// `prints_no_result` is the builder method.
    #[must_use]
    pub const fn get_prints_no_result(&self) -> bool {
        self.no_result
    }
}
