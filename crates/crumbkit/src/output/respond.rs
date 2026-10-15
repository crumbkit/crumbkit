//! The steps of [`finish_to`](super::finish_to) after a tool's handler
//! returns: the refusal of bad flags, the result, the failures, and the
//! dry-run line.

use std::io::{self, Write};

use super::budget::OverBudget;
use super::dry_run::DRY_RUN_LINE;
use super::failure::{self, PreparedFailure, Printing};
use super::result::{Answer, write_result};
use super::{Output, fields, problems};
use crate::{AsFailure, Document, Rows, Sink, Spelling, exit};

/// Prints the result to `sink` and returns the exit code as a number, as
/// [`finish_to`] describes, and whether the output budget cut a listing or a
/// list short.
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
) -> (u8, bool) {
    output.warn_without_command(&mut sink.err);
    let printing = output.printing();
    if let Some(refusal) = fields::flags_refusal(output, document) {
        let code = refusal.print(
            sink,
            output.json,
            spelling,
            printing,
            output.command.as_mut(),
        );
        return (code, false);
    }
    let value = match &result {
        Ok(value) => value,
        Err(error) => {
            let command = output.command.as_mut();
            failure::print(
                sink,
                output.json,
                spelling,
                printing,
                error.as_failure(),
                command,
            );
            return (printing.exit(error.as_failure().code()), false);
        }
    };
    // A batch's failures are read once, so its JSON document and its text
    // form print the same fields. Every other result has `None` here.
    // Each failure of a batch carries its key, and every other result has
    // no indexes, so its failures carry none.
    let failures: Option<Vec<PreparedFailure<'_>>> = value.failures().map(|failures| {
        let mut indexes = value.failure_indexes().iter();
        failures
            .into_iter()
            .map(|failure| {
                let key = indexes
                    .next()
                    .and_then(|&index| Some((index, value.key(index)?)));
                PreparedFailure::new(
                    failure,
                    spelling,
                    printing,
                    output.command.as_mut(),
                    &mut sink.err,
                )
                .keyed(key)
            })
            .collect()
    });
    // The exit reads every row that the result holds, including rows that
    // the output budget leaves unprinted.
    let problems = problems::count(value.rows());
    let written = write_result(
        &mut sink.out,
        &mut sink.err,
        output,
        spelling,
        document,
        value,
        failures.as_deref(),
        problems,
    );
    let (code, answer) = match written {
        Ok((answer, bytes)) => {
            output.spent = output.spent.saturating_add(bytes);
            (0, answer)
        }
        // A reader that stops early, as `shelf books list | head -1` does,
        // closes the pipe after it has read every line it wanted, so the
        // command did its job.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => (0, Answer::Whole),
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            (1, Answer::Whole)
        }
    };
    // The text and a document without the envelope have no `dry_run` field,
    // so a line on standard error marks a dry run, after any failures. A
    // result that was refused or failed to print gets no line.
    let dry_run_line = output.dry_run
        && (!output.json || !document.has_envelope())
        && code == 0
        && !matches!(answer, Answer::Refused(_));
    let (code, cut, left_out) = answered(sink, output, spelling, &answer, code);
    let failures = match &answer {
        Answer::Headed { oversize, .. } => failures.map(|listed| {
            let command = output.command.as_mut();
            with_oversize(listed, oversize, value, spelling, command, &mut sink.err)
        }),
        _ => failures,
    };
    let code = print_failures(sink, output, spelling, document, failures, left_out, code);
    // A failure decides the exit before a row that reports a problem, and a
    // reader that closed the pipe early still sees the problem's exit.
    let code = if code == 0 && problems > 0 {
        exit::problem_rows()
    } else {
        code
    };
    if dry_run_line {
        // Standard error is the last place to report anything, so a failed
        // write there goes unreported.
        let _ = writeln!(sink.err, "{DRY_RUN_LINE}");
    }
    (code, cut)
}

/// Prints the failures of a batch or a prose result in text, unless the
/// JSON document holds them in its envelope or in its in-order array, and the
/// failure of the rows that the budget left out, and returns the exit code:
/// the exit code of the first failure when there is any, which is 1 unless
/// the guide declares another for its code (ADR-0037), and `code`
/// otherwise.
fn print_failures<T: Rows, O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    failures: Option<Vec<PreparedFailure<'_>>>,
    left_out: Option<&OverBudget>,
    code: u8,
) -> u8 {
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
    // failures print as text even with `--json`, unless its array holds
    // them in input order.
    if !output.json || !(document.has_envelope() || document.prints_failures_in_order()) {
        for failure in &failures {
            // Standard error is the last place to report anything, so a
            // failure to write there goes unreported.
            let _ = failure::write_text(&mut sink.err, output.failure_form, failure);
        }
        if let Some(over) = left_out {
            let printing = output.printing();
            let command = output.command.as_mut();
            let over = PreparedFailure::new(over, spelling, printing, command, &mut sink.err);
            let _ = failure::write_text(&mut sink.err, output.failure_form, &over);
        }
    }
    match (failures.first(), left_out) {
        (Some(first), _) => output.printing().exit(first.failure().code()),
        (None, Some(over)) => output.printing().exit(over.code),
        (None, None) => code,
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
    code: u8,
) -> (u8, bool, Option<&'a OverBudget>) {
    match answer {
        Answer::Whole => (code, false, None),
        Answer::Cut(over) => {
            // A listing cut short exits 0, and the observer and the span
            // still receive its code (ADR-0046).
            crate::events::failed(spelling, over);
            #[cfg(feature = "tracing")]
            crate::trace::spans::failed(&spelling.spell(over));
            (code, true, None)
        }
        Answer::Partial(over) => (1, false, Some(over)),
        Answer::Headed { left_out, .. } => (code, false, left_out.as_ref()),
        Answer::Refused(over) => {
            let printing = output.printing();
            failure::print(
                sink,
                output.json,
                spelling,
                printing,
                over,
                output.command.as_mut(),
            );
            (printing.exit(over.code), false, None)
        }
    }
}

/// Returns the failures of a batch with the failure of each row of a headed
/// batch that was too large for the budget on its own, each keyed by its
/// row's key, in the order of the keys.
fn with_oversize<'a, T: Rows>(
    listed: Vec<PreparedFailure<'a>>,
    oversize: &'a [(usize, OverBudget)],
    value: &'a T,
    spelling: Spelling,
    mut command: Option<&mut clap::Command>,
    err: &mut impl Write,
) -> Vec<PreparedFailure<'a>> {
    let mut merged = Vec::with_capacity(listed.len() + oversize.len());
    let mut listed = listed.into_iter().peekable();
    for (position, over) in oversize {
        let before = |failure: &PreparedFailure<'_>| failure.index().is_some_and(|i| i < *position);
        while let Some(failure) = listed.next_if(before) {
            merged.push(failure);
        }
        // `code` is already the code that the run prints, so no rename
        // applies to it.
        let key = value.key(*position).map(|key| (*position, key));
        let printing = Printing::default();
        let command = command.as_deref_mut();
        merged.push(PreparedFailure::new(over, spelling, printing, command, err).keyed(key));
    }
    merged.extend(listed);
    merged
}
