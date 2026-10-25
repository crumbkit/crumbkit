//! How a result writes the JSON of a document without the envelope
//! (ADR-0016).

use std::io::{self, Write};

use super::Prepared;
use crate::output::envelope::{Selected, SelectedRows};
use crate::output::layout;
use crate::{Rows, Shape};

impl<T: Rows> Prepared<'_, T> {
    /// Writes the JSON of a document without the envelope with its first
    /// `shown` rows, or the whole result when `all` is set (ADR-0016).
    ///
    /// Without `--fields`, the whole result prints as its type serializes it,
    /// so a command prints an existing document unchanged, and its first
    /// rows print as an array. With `--fields`, only those fields of each row
    /// print, in that order, as an object for a single row and as an array
    /// otherwise.
    pub(super) fn write_unwrapped(
        &self,
        out: &mut impl Write,
        shown: usize,
        all: bool,
    ) -> io::Result<()> {
        let rows = self.value.rows();
        if self.view.names().is_none() {
            return match rows.get(..shown) {
                Some(first) if !all => layout::write(out, first, self.layout),
                _ => layout::write(out, self.value, self.layout),
            };
        }
        match self.values {
            [single] if T::SHAPE == Shape::Row => {
                let selected = Selected {
                    row: single,
                    columns: self.columns,
                };
                layout::write(out, &selected, self.layout)
            }
            list => {
                let selected = SelectedRows {
                    rows: list.get(..shown).unwrap_or(list),
                    columns: self.columns,
                };
                layout::write(out, &selected, self.layout)
            }
        }
    }
}
