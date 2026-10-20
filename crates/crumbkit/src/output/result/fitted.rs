//! How a result's final write fits its text table to the width that the
//! sink gives the result, which is the width of the terminal that standard
//! output is, after the output budget measured the unfitted table
//! (ADR-0121, ADR-0128).

use std::io;

use super::Prepared;
use crate::Rows;
use crate::output::budget::{self, Cut};
use crate::output::envelope::Failures;
use crate::sink::role::{Deliver, Role};

impl<T: Rows> Prepared<'_, T> {
    /// Measures the result as [`Prepared::write`] writes it, then delivers
    /// it to `sink` as [`Prepared::deliver_fitted`] does, and returns the
    /// number of bytes measured. The budget counts the unfitted table, so
    /// the rows that print never depend on the terminal.
    pub(super) fn measured_write(
        &mut self,
        sink: &mut dyn Deliver,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<usize> {
        let bytes = budget::measure(|counter| self.write(counter, shown, cut, failures))?;
        self.deliver_fitted(sink, bytes, shown, cut, failures)?;
        Ok(bytes)
    }

    /// Composes the result with its first `shown` rows as
    /// [`Prepared::write`] writes it, with the text table fitted to the
    /// width that `sink` gives the result (ADR-0121), and delivers it to
    /// `sink` as the result in one piece. `measured` is the number of bytes
    /// of the unfitted result, which the fitted result does not pass.
    fn deliver_fitted(
        &mut self,
        sink: &mut dyn Deliver,
        measured: usize,
        shown: usize,
        cut: Option<&Cut>,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        let mut piece = Vec::with_capacity(measured);
        self.fitting = true;
        let written = self.write(&mut piece, shown, cut, failures);
        self.fitting = false;
        written?;
        sink.deliver(Role::Result, &piece)
    }

    /// Delivers `measured`, the bytes that [`Prepared::write`] wrote for the
    /// whole result, to `sink` as the result. A text table that the sink
    /// gives a width is composed again fitted to that width.
    pub(super) fn write_measured(
        &mut self,
        sink: &mut dyn Deliver,
        measured: &[u8],
        shown: usize,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        if self.width.is_some() && self.table.is_some() {
            return self.deliver_fitted(sink, measured.len(), shown, None, failures);
        }
        sink.deliver(Role::Result, measured)
    }
}
