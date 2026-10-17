//! How a batch whose document answers each key left out prints when it does
//! not fit in the output budget: its first rows, then one over-budget
//! failure for each answer left out, at the position of its key (ADR-0068).

use std::io::{self, Write};

use serde::Serialize;

use super::batch::LeftOut;
use super::in_order::InOrderFailure;
use super::{Answer, Prepared};
use crate::output::budget::{self, Over, OverBudget};
use crate::output::envelope::{Failures, Selected};
use crate::output::failure::{Entry, PreparedFailure, Printing};
use crate::output::layout;
use crate::{Code, Rows};

/// Returns the failure of each answer that `value` leaves out when its first
/// `shown` rows print, with the answer's position among the keys, in key
/// order. Each failure has `code` and `budget`, and its recovery fetches the
/// answer's key alone.
pub(super) fn each_left_out<T: Rows>(
    value: &T,
    shown: usize,
    code: &'static Code,
    budget: usize,
) -> Vec<(usize, OverBudget)> {
    value
        .left_out_indexes(shown)
        .into_iter()
        .map(|position| {
            let fetch = value.key(position).and_then(|key| value.fetch_key(key));
            let over = Over::Key { fetch };
            (position, OverBudget { code, budget, over })
        })
        .collect()
}

impl<T: Rows> Prepared<'_, T> {
    /// Writes the most first rows of a batch that fit, and always its first,
    /// with one failure for each answer left out, and returns what it
    /// printed. A batch that leaves out no answer prints whole.
    ///
    /// In text, and in a document without the envelope that does not print
    /// its failures in input order, the failures go to standard error, which
    /// the budget does not count. In a JSON document that holds them, each
    /// failure counts against the budget, and a row that prints takes the
    /// place of its failure, so the document with more rows can be the
    /// shorter one. [`Prepared::fit_each`] then finds the count.
    ///
    /// It returns `None` when even the first row and the failure of every
    /// later answer pass the budget, so the batch ends with the one failure
    /// that names the answers left out.
    pub(super) fn answer_each(
        &mut self,
        out: &mut impl Write,
        err: &mut impl Write,
        listed: &[PreparedFailure<'_>],
        left_out: &mut LeftOut<'_>,
        in_document: bool,
    ) -> io::Result<Option<(Answer, usize)>> {
        let value = self.value;
        let rows = value.rows().len();
        let low = rows.min(1);
        let (code, budget) = (left_out.code, left_out.budget);
        let indexes = value.failure_indexes();
        if !in_document {
            let shown = budget::largest(low, rows, |shown| self.fits(shown, listed, None, budget))?;
            let failures = Failures::of(listed, indexes, self.form);
            let bytes = self.measured_write(out, shown, None, Some(failures))?;
            let each = each_left_out(value, shown, code, budget);
            if each.is_empty() {
                self.send_notes(err, &mut Vec::new(), rows, None);
                return Ok(Some((Answer::Whole, bytes)));
            }
            let answer = Answer::Keyed {
                each,
                left_out: None,
            };
            return Ok(Some((answer, bytes)));
        }
        let mut each = each_left_out(value, low, code, budget);
        let several = crate::output::several_keys(value);
        let prepared: Vec<(usize, PreparedFailure<'_>)> = each
            .iter()
            .map(|(position, over)| {
                let key = value.key(*position).map(|key| (*position, key));
                // `code` is already the code that the run prints, so no
                // rename applies to it.
                let failure = PreparedFailure::new(
                    over,
                    left_out.spelling,
                    Printing::default(),
                    left_out.command.as_deref_mut(),
                    err,
                );
                (*position, failure.keyed(key, several))
            })
            .collect();
        let Some(shown) = self.fit_each(listed, &prepared, low, budget)? else {
            return Ok(None);
        };
        let from = shown - low;
        let failures = Failures {
            each: prepared.get(from..).unwrap_or_default(),
            ..Failures::of(listed, indexes, self.form)
        };
        let bytes = self.measured_write(out, shown, None, Some(failures))?;
        let each = each.split_off(from.min(each.len()));
        let answer = Answer::Keyed {
            each,
            left_out: None,
        };
        Ok(Some((answer, bytes)))
    }

    /// Returns the most rows from `low` up that fit in `budget` in the JSON
    /// document beside the batch's own failures `listed` and the failures
    /// in `each` of the answers that the rows after them leave out, or
    /// `None` when no count fits.
    ///
    /// It measures the document without rows or failures once, and each
    /// row and each failure once as an element of its array in the
    /// document's layout and at its depth, so the size of each count is a
    /// sum.
    fn fit_each(
        &mut self,
        listed: &[PreparedFailure<'_>],
        each: &[(usize, PreparedFailure<'_>)],
        low: usize,
        budget: usize,
    ) -> io::Result<Option<usize>> {
        let empty = Failures::of(&[], &[], self.form);
        let empty = budget::measure(|counter| self.write(counter, 0, None, Some(empty)))?;
        let in_order = self.document.prints_failures_in_order();
        let depth = if self.document.has_envelope() { 2 } else { 1 };
        // `rows[k]` is the bytes of the first `k` rows.
        let mut rows = vec![0_usize];
        for (number, row) in self.value.rows().iter().enumerate() {
            let bytes = match self.values.get(number) {
                Some(selected) => self.element(
                    &Selected {
                        row: selected,
                        columns: self.columns,
                    },
                    depth,
                )?,
                None => self.element(row, depth)?,
            };
            rows.push(rows.last().copied().unwrap_or(0).saturating_add(bytes));
        }
        let own = listed.iter().try_fold(0_usize, |sum, failure| {
            Ok::<_, io::Error>(sum.saturating_add(self.failure_len(failure, depth)?))
        })?;
        let lens = each
            .iter()
            .map(|(_, failure)| self.failure_len(failure, depth))
            .collect::<io::Result<Vec<usize>>>()?;
        // `left[i]` is the bytes of the failures in `each` from the `i`th.
        let mut left: Vec<usize> = lens
            .iter()
            .rev()
            .scan(0_usize, |sum, &bytes| {
                *sum = sum.saturating_add(bytes);
                Some(*sum)
            })
            .collect();
        left.reverse();
        left.push(0);
        let shown = (low..rows.len()).rev().find(|&shown| {
            let from = shown - low;
            let failures = listed.len() + each.len().saturating_sub(from);
            let row_bytes = rows.get(shown).copied().unwrap_or(0);
            let failure_bytes = own.saturating_add(left.get(from).copied().unwrap_or(0));
            let arrays = if in_order {
                let bytes = row_bytes.saturating_add(failure_bytes);
                layout::array_len(shown + failures, bytes, self.layout, depth)
            } else {
                layout::array_len(shown, row_bytes, self.layout, depth).saturating_add(
                    layout::array_len(failures, failure_bytes, self.layout, depth),
                )
            };
            empty.saturating_add(arrays) <= budget
        });
        Ok(shown)
    }

    /// Returns the bytes of `value` as an element of an array `depth` levels
    /// deep in the document's layout.
    fn element(&self, value: &impl Serialize, depth: usize) -> io::Result<usize> {
        layout::element_len(value, self.layout, depth)
    }

    /// Returns the bytes of `failure` as an element of the document's array
    /// of failures, `depth` levels deep: the entry of `failures`, or the
    /// failure document of an array in input order.
    fn failure_len(&self, failure: &PreparedFailure<'_>, depth: usize) -> io::Result<usize> {
        if self.document.prints_failures_in_order() {
            return self.element(&InOrderFailure(failure, self.form), depth);
        }
        match self.form {
            Some(form) => self.element(&form.entry(failure), depth),
            None => self.element(&Entry(failure), depth),
        }
    }
}
