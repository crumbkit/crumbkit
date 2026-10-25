//! How a result's final write fits its text table to the width that the
//! sink gives the result, after the output budget measured the unfitted
//! table. `finish` gives its sink the width of the terminal that standard
//! output is, and a tool gives its own sink a width with `Sink::fitted`
//! (ADR-0121, ADR-0128, ADR-0134). The same final write gives the table's
//! cells their styles, which the budget does not count either (ADR-0150).

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
    /// width that `sink` gives the result (ADR-0121) and its cells in their
    /// styles when the sink wants styles (ADR-0150), and delivers it to
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
        self.final_write = true;
        let written = self.write(&mut piece, shown, cut, failures);
        self.final_write = false;
        written?;
        sink.deliver(Role::Result, &piece)
    }

    /// Delivers `measured`, the bytes that [`Prepared::write`] wrote for the
    /// whole result, to `sink` as the result. A text table that the sink
    /// gives a width, or whose cells have styles, is composed again fitted
    /// to that width and with those styles (ADR-0150).
    pub(super) fn write_measured(
        &mut self,
        sink: &mut dyn Deliver,
        measured: &[u8],
        shown: usize,
        failures: Option<Failures<'_>>,
    ) -> io::Result<()> {
        let decorated = self
            .table
            .as_ref()
            .is_some_and(|table| self.width.is_some() || table.is_styled());
        if decorated {
            return self.deliver_fitted(sink, measured.len(), shown, None, failures);
        }
        sink.deliver(Role::Result, measured)
    }
}
