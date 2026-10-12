//! The steps of [`finish_to`](super::finish_to) after a tool's handler
//! returns: the refusal of bad flags, the result, the failures, and the
//! dry-run line.

use std::io::{self, Write};
use std::process::ExitCode;

use super::budget::OverBudget;
use super::dry_run::DRY_RUN_LINE;
use super::failure::{self, PreparedFailure};
use super::result::{Answer, write_result};
use super::{Output, fields};
use crate::{AsFailure, Document, Rows, Sink, Spelling};

/// Prints the result to `sink` and returns the exit code, as [`finish_to`]
/// describes, and whether the output budget cut a listing or a list short.
#[expect(
    clippy::needless_pass_by_value,
    reason = "finish_to takes the result by value, as finish does, so a tool passes the result it built in the same way to either"
)]
pub(super) fn respond<T: Rows, E: AsFailure, O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    result: Result<T, E>,
) -> (ExitCode, bool) {
    if let Some(refusal) = fields::flags_refusal::<T>(output) {
        let form = output.failure_form;
        let code = refusal.print(sink, output.json, spelling, form, output.command.as_mut());
        return (code, false);
    }
    let value = match &result {
        Ok(value) => value,
        Err(error) => {
            let command = output.command.as_mut();
            let form = output.failure_form;
            failure::print(
                sink,
                output.json,
                spelling,
                form,
                error.as_failure(),
                command,
            );
            return (ExitCode::FAILURE, false);
        }
    };
    // A batch's failures are read once, so its JSON document and its text
    // form print the same fields. Every other result has `None` here.
    let failures: Option<Vec<PreparedFailure<'_>>> = value.failures().map(|failures| {
        failures
            .into_iter()
            .map(|failure| {
                PreparedFailure::new(failure, spelling, output.command.as_mut(), &mut sink.err)
            })
            .collect()
    });
    let written = write_result(
        &mut sink.out,
        &mut sink.err,
        output,
        spelling,
        document,
        value,
        failures.as_deref(),
    );
    let (code, answer) = match written {
        Ok((answer, bytes)) => {
            output.spent = output.spent.saturating_add(bytes);
            (ExitCode::SUCCESS, answer)
        }
        // A reader that stops early, as `shelf books list | head -1` does,
        // closes the pipe after it has read every line it wanted, so the
        // command did its job.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {
            (ExitCode::SUCCESS, Answer::Whole)
        }
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            (ExitCode::FAILURE, Answer::Whole)
        }
    };
    // The text and a document without the envelope have no `dry_run` field,
    // so a line on standard error marks a dry run, after any failures. A
    // result that was refused or failed to print gets no line.
    let dry_run_line = output.dry_run
        && (!output.json || !document.has_envelope())
        && code == ExitCode::SUCCESS
        && !matches!(answer, Answer::Refused(_));
    let (code, cut, left_out) = answered(sink, output, spelling, &answer, code);
    let code = print_failures(sink, output, spelling, document, failures, left_out, code);
    if dry_run_line {
        // Standard error is the last place to report anything, so a failed
        // write there goes unreported.
        let _ = writeln!(sink.err, "{DRY_RUN_LINE}");
    }
    (code, cut)
}

/// Prints the failures of a batch or a prose result in text, and the
/// failure of the rows that the budget left out, and returns the exit code:
/// 1 when there is any failure, and `code` otherwise.
fn print_failures<T: Rows, O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    failures: Option<Vec<PreparedFailure<'_>>>,
    left_out: Option<&OverBudget>,
    code: ExitCode,
) -> ExitCode {
    let Some(failures) = failures else {
        return code;
    };
    // The span records only the batch's first failure, as the usage log does.
    #[cfg(feature = "tracing")]
    match (failures.first(), left_out) {
        (Some(first), _) => crate::trace::spans::failed(first.code()),
        (None, Some(over)) => crate::trace::spans::failed(&spelling.spell(over.code)),
        (None, None) => {}
    }
    // A document without the envelope has no `failures` field, so its
    // failures print as text even with `--json`.
    if !output.json || !document.has_envelope() {
        for failure in &failures {
            // Standard error is the last place to report anything, so a
            // failure to write there goes unreported.
            let _ = failure::write_text(&mut sink.err, output.failure_form, failure);
        }
        if let Some(over) = left_out {
            let command = output.command.as_mut();
            let over = PreparedFailure::new(over, spelling, command, &mut sink.err);
            let _ = failure::write_text(&mut sink.err, output.failure_form, &over);
        }
    }
    if failures.is_empty() {
        code
    } else {
        ExitCode::FAILURE
    }
}

/// Prints the failure of a result that `write_result` refused, and returns
/// the exit code after `answer`, whether the budget cut a listing or a list
/// short, and the failure that ends a batch whose later rows were left out.
fn answered<'a, O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    output: &mut Output,
    spelling: Spelling,
    answer: &'a Answer,
    code: ExitCode,
) -> (ExitCode, bool, Option<&'a OverBudget>) {
    match answer {
        Answer::Whole => (code, false, None),
        Answer::Cut(over) => {
            // A listing cut short exits 0, and the usage log and the span
            // still record its code (ADR-0046).
            #[cfg(feature = "usage-log")]
            crate::usage::failed(spelling, over);
            #[cfg(feature = "tracing")]
            crate::trace::spans::failed(&spelling.spell(over));
            #[cfg(not(any(feature = "usage-log", feature = "tracing")))]
            let _ = (over, spelling);
            (code, true, None)
        }
        Answer::Partial(over) => (ExitCode::FAILURE, false, Some(over)),
        Answer::Refused(over) => {
            let form = output.failure_form;
            failure::print(
                sink,
                output.json,
                spelling,
                form,
                over,
                output.command.as_mut(),
            );
            (ExitCode::FAILURE, false, None)
        }
    }
}
