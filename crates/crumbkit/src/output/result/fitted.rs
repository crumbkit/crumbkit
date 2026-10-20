//! How a result's final write fits its text table to the width of the
//! terminal that standard output is, after the output budget measured the
//! unfitted table (ADR-0121).

use std::io::{self, Write};

use super::Prepared;
use crate::Rows;
use crate::output::budget::{self, Cut};
use crate::output::envelope::Failures;

impl<T: Rows> Prepared<'_, T> {
    /// Measures the result as [`Prepared::write`] writes it, then writes
    /// it as [`Prepared::write_fitted`] does, and returns the number of
    /// bytes measured. The budget counts the unfitted table, so the rows
    /// that print never depend on the terminal.
    pub(super) fn measured_write(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<usize> {
        let bytes = budget::measure(|counter| self.write(counter, shown, cut, failures))?;
        self.write_fitted(out, shown, cut, failures)?;
        Ok(bytes)
    }

    /// Writes the result with its first `shown` rows to `out`, as
    /// [`Prepared::write`] does, with the text table fitted to the
    /// terminal's width when standard output is a terminal (ADR-0121).
    fn write_fitted(
        &mut self,
        out: &mut impl Write,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        self.fitting = true;
        let written = self.write(out, shown, cut, failures);
        self.fitting = false;
        written
    }

    /// Writes `measured`, the bytes that [`Prepared::write`] wrote for the
    /// whole result, to `out` and flushes it. A text table at a terminal is
    /// written again fitted to the terminal's width.
    pub(super) fn write_measured(
        &mut self,
        out: &mut impl Write,
        measured: &[u8],
        shown: usize,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        if self.width.is_some() && self.table.is_some() {
            return self.write_fitted(out, shown, None, failures);
        }
        out.write_all(measured)?;
        out.flush()
    }
}
