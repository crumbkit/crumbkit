//! How a stream writes one item to its line.

use std::borrow::Cow;
use std::io::{self, Write};

use super::Streaming;
use crate::Row;
use crate::output::envelope::{Extras, Selected, write_document};
use crate::output::{JsonLayout, lines, table, unwrapped};

impl<R: Row, O: Write, W: Write> Streaming<'_, R, O, W> {
    /// Writes `item` to the line, in the format that the flags choose.
    pub(super) fn render(&mut self, item: &R) -> io::Result<()> {
        let line = &mut self.line;
        if self.json && !self.document.get_envelope().wraps() {
            return unwrapped::write_json(line, self.fields, item);
        }
        if self.json && self.full {
            // Serializing the item directly keeps every field in its
            // declaration order.
            return write_document(
                line,
                self.document,
                self.dry_run,
                JsonLayout::OneLine,
                item,
                &Extras::none(),
            );
        }
        if self.lined {
            line.extend_from_slice(lines::line(item)?.as_bytes());
            return Ok(());
        }
        let row = serde_json::to_value(item)?;
        if self.json {
            let selected = Selected {
                row: &row,
                columns: &self.columns,
            };
            write_document(
                line,
                self.document,
                self.dry_run,
                JsonLayout::OneLine,
                &selected,
                &Extras::none(),
            )
        } else {
            let cells: Vec<Cow<'_, str>> = self
                .columns
                .iter()
                .map(|name| table::cell(row.get(name)))
                .collect();
            table::write_line(line, &cells, &self.widths)
        }
    }
}
