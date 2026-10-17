//! How a batch that does not fit in the output budget prints: its first
//! rows, then one failure that names the keys left out (ADR-0046).

use std::io::{self, Write};

use clap::Command;

use super::rerun::{self, Rerun};
use super::{Answer, Prepared};
use crate::output::budget::{self, Over, OverBudget};
use crate::output::envelope::Failures;
use crate::output::failure::{PreparedFailure, Printing};
use crate::{Code, Rows, Spelling};

/// What the failure of a batch's left-out keys needs beside the batch.
pub(super) struct LeftOut<'c> {
    /// The spelling of the failure's code.
    pub(super) spelling: Spelling,
    /// The tool's command, which the failure's call renders against.
    pub(super) command: &'c mut Command,
    /// The over-budget code that the run prints.
    pub(super) code: &'static Code,
    /// The bytes that the result had to fit in.
    pub(super) budget: usize,
    /// The caller's words split at the place of the keys, which the
    /// recoveries rerun with the keys left out, or `None` when the
    /// recoveries name the batch's command alone.
    pub(super) rerun: Option<Rerun>,
}

impl<T: Rows> Prepared<'_, T> {
    /// Writes the most first rows of a batch that fit, and always its first,
    /// with the failure of the keys left out after the batch's own failures
    /// in the JSON document. A batch whose every key was answered and whose
    /// rows all print has no such failure and prints whole.
    ///
    /// The failure in the JSON document counts against the budget, so it
    /// names as many keys as fit: the rows that print are the most that fit
    /// beside a failure that names every key left out after the first row.
    /// When even the first row and that failure pass the budget, the failure
    /// names the most first keys that fit and counts the rest under
    /// `unlisted`. When the first row and a failure that names no key pass
    /// the budget, the failure names every key, because the document passes
    /// the budget whichever keys it names (ADR-0070). In text, and in a
    /// document without the envelope, the failure goes to
    /// standard error, which the budget does not count, so it names every
    /// key.
    pub(super) fn partial(
        &mut self,
        out: &mut impl Write,
        err: &mut impl Write,
        listed: &[PreparedFailure<'_>],
        mut left_out: LeftOut<'_>,
    ) -> io::Result<(Answer, usize)> {
        let rows = self.value.rows().len();
        let in_document = self.json && self.document.get_envelope().wraps();
        let (shown, over) = if in_document {
            self.fit_failure(err, listed, &mut left_out)?
        } else {
            let low = rows.min(1);
            let shown = budget::largest(low, rows, |shown| {
                self.fits(shown, listed, None, left_out.budget)
            })?;
            (shown, self.left_out_failure(&left_out, shown, usize::MAX))
        };
        let failures = |over| Failures { listed, over };
        if let Over::Batch {
            left_out: keys,
            unlisted: 0,
            ..
        } = &over.over
            && keys.is_empty()
        {
            let bytes = self.measured_write(out, rows, None, Some(failures(None)))?;
            self.send_notes(err, &mut Vec::new(), rows, None);
            return Ok((Answer::Whole, bytes));
        }
        let bytes = {
            // `code` is already the code that the run prints, so no rename
            // applies to it.
            let prepared = in_document.then(|| {
                PreparedFailure::new(
                    &over,
                    left_out.spelling,
                    Printing::default(),
                    left_out.command,
                    err,
                )
            });
            self.measured_write(out, shown, None, Some(failures(prepared.as_ref())))?
        };
        self.send_notes_before_left_out(err, shown);
        Ok((Answer::Partial(over), bytes))
    }

    /// Returns the number of rows that print in the JSON document and the
    /// failure that names the keys left out, so the two fit in the budget
    /// when the first row allows it.
    ///
    /// Each count of rows is measured beside the failure that names every
    /// key left out after the first row, whose list is the longest, so the
    /// failure that prints is never longer than the one measured and a
    /// binary search finds the count. When the first row does not fit
    /// beside that failure, the first row prints, and the failure names the
    /// most first keys that fit, or every key when none fits. Naming one more key adds at least the key's
    /// quotes and comma, which outweigh the two digits that the count of
    /// the rest can lose in `unlisted` and in the message, so a binary
    /// search finds that number too. The search stops one key short of every
    /// key, so the count of the rest is never 0 and the message never drops
    /// its sentence about the rest.
    fn fit_failure(
        &mut self,
        err: &mut impl Write,
        listed: &[PreparedFailure<'_>],
        left_out: &mut LeftOut<'_>,
    ) -> io::Result<(usize, OverBudget)> {
        let rows = self.value.rows().len();
        let low = rows.min(1);
        let budget = left_out.budget;
        let most = self.left_out_failure(left_out, low, usize::MAX);
        let prepared = PreparedFailure::new(
            &most,
            left_out.spelling,
            Printing::default(),
            &mut *left_out.command,
            err,
        );
        if self.fits(low, listed, Some(&prepared), budget)? {
            let shown = budget::largest(low, rows, |shown| {
                self.fits(shown, listed, Some(&prepared), budget)
            })?;
            return Ok((shown, self.left_out_failure(left_out, shown, usize::MAX)));
        }
        let keys = match &most.over {
            Over::Batch { left_out, .. } => left_out.len(),
            Over::Row | Over::Whole | Over::Stream => 0,
        };
        let mut fits_naming = |named: usize| {
            let failure = self.left_out_failure(left_out, low, named);
            let prepared = PreparedFailure::new(
                &failure,
                left_out.spelling,
                Printing::default(),
                &mut *left_out.command,
                err,
            );
            self.fits(low, listed, Some(&prepared), budget)
        };
        // The document passes the budget whichever keys the failure names,
        // so it names every key and keeps the recovery that fetches them.
        if !fits_naming(0)? {
            return Ok((low, most));
        }
        // Every key was measured above, so the search stops one short of it.
        let named = budget::largest(0, keys.saturating_sub(1), fits_naming)?;
        Ok((low, self.left_out_failure(left_out, low, named)))
    }

    /// Returns whether the result with its first `shown` rows, the batch's
    /// own failures and `over` fits in `budget`.
    pub(super) fn fits(
        &mut self,
        shown: usize,
        listed: &[PreparedFailure<'_>],
        over: Option<&PreparedFailure<'_>>,
        budget: usize,
    ) -> io::Result<bool> {
        let failures = Failures { listed, over };
        let bytes = budget::measure(|counter| self.write(counter, shown, None, Some(failures)))?;
        Ok(bytes <= budget)
    }

    /// Returns the failure of the keys that the batch leaves out when its
    /// first `shown` rows print, which names the first `named` of them and
    /// counts the rest, and the answers without a key, under `unlisted`.
    fn left_out_failure(&self, left_out: &LeftOut<'_>, shown: usize, named: usize) -> OverBudget {
        let keys = self.value.left_out(shown);
        let listed: Vec<String> = keys
            .iter()
            .flatten()
            .take(named)
            .map(ToString::to_string)
            .collect();
        OverBudget {
            code: left_out.code,
            budget: left_out.budget,
            over: Over::Batch {
                unlisted: keys.len().saturating_sub(listed.len()),
                left_out: listed,
                from: self.value.left_out_from(shown),
                fetch: rerun::fetch_all(self.value, left_out.rerun.as_ref()),
            },
        }
    }
}
