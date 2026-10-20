//! The steps of [`finish_to`](super::finish_to) after a tool's handler
//! returns: the refusal of bad flags, the handler's failure or the result,
//! the result's failures, and the dry-run line. Each piece is delivered
//! with its role, and the sink maps the role to a stream (ADR-0128).

use std::io;

use super::budget::OverBudget;
use super::dry_run::DRY_RUN_LINE;
use super::failure::{self, PreparedFailure, Printing};
use super::result::{Answer, write_result};
use super::{Output, fields};
use crate::sink::role::{Deliver, Message, Role};
use crate::{AsFailure, Document, Rows, Spelling, exit};

/// Prints the result to `sink` and returns the exit code as a number, as
/// [`finish_to`] describes, and whether the output budget cut a listing or a
/// list short.
#[expect(
    clippy::needless_pass_by_value,
    reason = "finish_to takes the result by value, as finish does, so a tool passes the result it built in the same way to either"
)]
pub(super) fn respond<T: Rows, E: AsFailure>(
    sink: &mut dyn Deliver,
    output: &mut Output,
    spelling: Spelling,
    document: Document<T>,
    result: Result<T, E>,
) -> (u8, bool) {
    let printing = output.printing();
    // A tool may finish an early failure in another document than its leaf
    // declares, so a failure is not checked against the document. The
    // `Output` holds only a view that every printer accepts (ADR-0093).
    if result.is_ok()
        && let Some(refusal) = fields::flags_refusal(output, document)
    {
        let code = refusal.print(
            sink,
            output.is_json(),
            spelling,
            printing,
            &mut output.command,
        );
        return (code, false);
    }
    let value = match &result {
        Ok(value) => value,
        Err(error) => {
            let json = output.is_json();
            let command = &mut output.command;
            failure::print(sink, json, spelling, printing, error.as_failure(), command);
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
                PreparedFailure::new(failure, spelling, printing, &mut output.command, sink)
                    .keyed(key)
            })
            .collect()
    });
    // The exit reads every row that the result holds, including rows that
    // the output budget leaves unprinted, and a listing that
    // `Listing::from_all` cut to its limit also counts the rows it cut.
    let problems = value.problems();
    let written = write_result(
        sink,
        output,
        spelling,
        document,
        value,
        failures.as_deref(),
        problems,
    );
    let (code, answer) = match written {
        Ok((answer, _bytes)) => (0, answer),
        // A reader that stops early, as `shelf books list | head -1` does,
        // closes the pipe after it has read every line it wanted, so the
        // command did its job.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => (0, Answer::Whole),
        Err(error) => {
            sink.report_failed_write(&error);
            (1, Answer::Whole)
        }
    };
    // The text and a document without the envelope have no `dry_run` field,
    // so a dry-run message marks a dry run, after any failures. A
    // result that was refused or failed to print gets no line.
    let dry_run_line = output.dry_run
        && (!output.is_json() || !document.get_envelope().wraps())
        && code == 0
        && !matches!(answer, Answer::Refused(_));
    let (code, cut, left_out) = answered(sink, output, spelling, &answer, code);
    let failures = match &answer {
        Answer::RefusedFirst {
            position, first, ..
        } => failures.map(|mut listed| {
            // `code` is already the code that the run prints, so no rename
            // applies to it.
            let key = value.key(*position).map(|key| (*position, key));
            let command = &mut output.command;
            let printing = Printing::default();
            let refused = PreparedFailure::new(first, spelling, printing, command, sink);
            listed.insert(failure::before(&listed, *position), refused.keyed(key));
            listed
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
        // A message is the last thing to report, so a failure to deliver it
        // goes unreported.
        let line = format!("{DRY_RUN_LINE}\n");
        let _ = sink.deliver(Role::Message(Message::DryRun), line.as_bytes());
    }
    (code, cut)
}

/// Prints the failures of a batch or a prose result in text, unless the
/// JSON document holds them in its envelope, and the
/// failure of the rows that the budget left out, and returns the exit code:
/// the exit code of the first failure when there is any, which is 1 unless
/// the guide declares another for its code (ADR-0037), and `code`
/// otherwise.
fn print_failures<T: Rows>(
    sink: &mut dyn Deliver,
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
    // failures print as text even with `--json`.
    if !output.is_json() || !document.get_envelope().wraps() {
        for failure in &failures {
            failure::deliver_text(sink, failure);
        }
        if let Some(over) = left_out {
            let printing = output.printing();
            let command = &mut output.command;
            let over = PreparedFailure::new(over, spelling, printing, command, sink);
            failure::deliver_text(sink, &over);
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
fn answered<'a>(
    sink: &mut dyn Deliver,
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
        // The failure of the refused answer decides the exit.
        Answer::RefusedFirst { left_out, .. } => (code, false, left_out.as_ref()),
        Answer::Refused(over) => {
            let printing = output.printing();
            failure::print(
                sink,
                output.is_json(),
                spelling,
                printing,
                over,
                &mut output.command,
            );
            (printing.exit(over.code), false, None)
        }
    }
}
