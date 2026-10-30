//! How a listing or a list that passes the output budget prints its first
//! rows, marked as cut short (ADR-0046).

use std::io::{self, Write};

use super::{Answer, Prepared};
use crate::output::budget::{self, Cut, Over, OverBudget};
use crate::sink::role::Deliver;
use crate::{Code, Rows, Shape};

impl<T: Rows> Prepared<'_, T> {
    /// Writes the most first rows of a listing or a list that fit in
    /// `budget`, marked as cut short with `code`, which `spelled` spells.
    /// A document that always prints its first answer prints at least one
    /// row.
    pub(super) fn cut(
        &mut self,
        sink: &mut dyn Deliver,
        spelled: String,
        code: &'static Code,
        budget: usize,
    ) -> io::Result<(Answer, usize)> {
        let rows = self.rows;
        let cut = Cut {
            code: spelled,
            budget,
        };
        let low = usize::from(self.document.get_always_print_first());
        let mut fits = |shown| {
            let bytes = budget::measure(|counter| self.write(counter, shown, Some(&cut), None))?;
            Ok(bytes <= budget)
        };
        let shown = budget::largest(low, rows - 1, &mut fits)?;
        if shown == 0 && !fits(0)? {
            // Even the result without rows passes the budget, as a long
            // summary can, so the result is refused as a single row is.
            let over = OverBudget {
                code,
                budget,
                over: Over::Row,
            };
            return Ok((Answer::Refused(over), 0));
        }
        let bytes = self.measured_write(sink, shown, Some(&cut), None)?;
        self.send_notes(sink, &mut Vec::new(), shown, Some(&cut));
        Ok((Answer::Cut(code), bytes))
    }

    /// Writes the line that says the budget cut the result short to `out`.
    pub(super) fn write_cut_line(
        &self,
        out: &mut impl Write,
        shown: usize,
        cut: &Cut,
    ) -> io::Result<()> {
        let listing = T::SHAPE == Shape::Listing;
        // A list holds every row it has, and a listing without a total has
        // no count to give.
        let total = match self.value.total() {
            None if !listing => Some(self.value.rows().len()),
            total => total,
        };
        budget::write_cut_line(
            out,
            shown,
            total,
            self.value.description(),
            self.value.singular(),
            cut.budget,
            listing,
        )
    }
}
