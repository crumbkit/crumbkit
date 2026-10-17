//! `Room`, which counts the bytes that `finish` prints for the rows of a
//! batch as its handler pushes them, so the handler stops fetching keys once
//! the rows fill the output budget (ADR-0068).

use std::borrow::Cow;
use std::io;

use serde::Serialize;
use serde_json::Value;

use super::budget::{self, Counter};
use super::envelope::{Extras, Failures, Selected, write_document};
use super::table::cell;
use super::{JsonLayout, Output, columns, layout, lines, result};
use crate::{AsFailure, Batch, Document, Row, Rows};

/// The bytes that `finish` prints for the rows of a batch, which
/// [`Output::room`] returns before the handler fetches the first key.
///
/// [`Room::has_room`] measures each row that the handler pushed since the
/// last call once, in the format that the run prints: the JSON document in
/// the guide's layout with the columns that the view flags choose, the text
/// table with each column as wide as its widest cell, each row's own line
/// for a row type that declares `Row::LINE`, or the bytes and the `==> KEY <==` headers of a headed batch. The count is the bytes that
/// `finish` prints for those rows when the batch has no failure, because
/// each failure only adds bytes. A handler that stops when `has_room`
/// returns false therefore never stops before the row at which `finish`
/// would cut the batch.
///
/// ```
/// # #[derive(serde::Serialize, schemars::JsonSchema)]
/// # struct Book { id: u32 }
/// # impl crumbkit::Row for Book { const COLUMNS: &'static [&'static str] = &["id"]; }
/// use crumbkit::{Batch, Document, Error, Output};
///
/// const BOOKS_SHOW: Document<Batch<Book, Error>> =
///     Document::new("shelf", "books-show", 1, "The books with the given ids.");
///
/// fn show(output: &Output, ids: &[u32]) -> Batch<Book, Error> {
///     let mut room = output.room(BOOKS_SHOW);
///     let mut batch = Batch::new(&["books", "show"], "ids", ids);
///     for &id in ids {
///         if !room.has_room(&batch) {
///             break;
///         }
///         batch.push(Ok(Book { id }));
///     }
///     batch
/// }
/// # let output = Output::new(
/// #     crumbkit::ViewFlags::new(),
/// #     clap::Command::new("shelf"),
/// #     &crumbkit::help::Guide::new(),
/// #     ["shelf"],
/// # );
/// # let _ = show(&output, &[2, 7, 9]);
/// ```
#[derive(Debug)]
pub struct Room<T> {
    /// The document that the batch prints as.
    document: Document<T>,
    /// The bytes that the run can still print.
    budget: usize,
    /// The columns that the table and the JSON show, or none when each row
    /// prints with every field.
    columns: Vec<String>,
    /// How the rows print, with the counts of the rows measured so far.
    form: Form,
    /// The number of rows measured so far.
    measured: usize,
}

/// How the rows of a batch print, and what [`Room`] keeps of the rows that
/// it measured.
#[derive(Debug)]
enum Form {
    /// The JSON document, whose rows sit `depth` levels deep.
    Json {
        /// The guide's layout, or `None` for one line.
        layout: JsonLayout,
        /// How deep each row sits in the document.
        depth: usize,
        /// The bytes of the document without rows.
        empty: usize,
        /// The bytes of the rows measured, each in the layout at its depth.
        rows: usize,
    },
    /// The text table.
    Table {
        /// The width of each column over the header and the rows measured.
        widths: Vec<usize>,
        /// For each column, the number of lines that print a cell after it,
        /// each of which pads that column to its width.
        padded: Vec<usize>,
        /// The bytes of the lines measured beside the padding of each
        /// column: the text of their cells and their line feeds.
        text: usize,
    },
    /// The text of a row type that prints each row's own line (ADR-0064),
    /// with the bytes of the lines measured.
    Lines(usize),
    /// The text of a headed batch.
    Headed {
        /// Whether each row prints under a header.
        headers: bool,
        /// The bytes of the headers of the rows that print.
        header_bytes: usize,
        /// The bytes of the rows that print.
        content: usize,
        /// The number of rows that print.
        printed: usize,
    },
}

impl Output {
    /// Returns the [`Room`] of a batch that prints as `document`, which
    /// counts the bytes that `finish` prints for the batch's rows within the
    /// bytes that [`Output::budget`] has left now (ADR-0068).
    pub fn room<R: Row, E: AsFailure>(&self, document: Document<Batch<R, E>>) -> Room<Batch<R, E>> {
        let columns: Vec<String> = columns::columns(self, document, false)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let form = if self.json {
            let depth = if document.get_envelope().wraps() {
                2
            } else {
                1
            };
            let empty = empty_document(document, self.dry_run, self.json_layout);
            Form::Json {
                layout: self.json_layout,
                depth,
                // A document that fails to serialize prints nothing, and
                // `finish` reports the error.
                empty: empty.unwrap_or(0),
                rows: 0,
            }
        } else if document.is_headed() {
            Form::Headed {
                headers: false,
                header_bytes: 0,
                content: 0,
                printed: 0,
            }
        } else if lines::chosen::<R>(self) {
            Form::Lines(0)
        } else {
            let header: Vec<Cow<'_, str>> = columns
                .iter()
                .map(|name| Cow::Owned(name.to_uppercase()))
                .collect();
            let mut table = Form::Table {
                widths: vec![0; columns.len()],
                padded: vec![0; columns.len()],
                text: 0,
            };
            table.add_line(&header);
            table
        };
        Room {
            document,
            budget: self.budget(),
            columns,
            form,
            measured: 0,
        }
    }
}

impl<T: Rows> Room<T> {
    /// Measures each row of `value` that the last call did not measure, and
    /// returns whether the rows measured so far take fewer bytes than the
    /// run can print, so the handler can fetch the next key.
    ///
    /// A row that fails to serialize counts the bytes written before the
    /// error, and `finish` reports the error when it prints.
    pub fn has_room(&mut self, value: &T) -> bool {
        let rows = value.rows();
        let columns: Vec<&str> = self.columns.iter().map(String::as_str).collect();
        let budget = self.budget;
        let first_always = self.document.always_prints_first();
        let bytes_of = self.document.headed_bytes();
        for (index, row) in rows.iter().enumerate().skip(self.measured) {
            match &mut self.form {
                Form::Json {
                    layout,
                    depth,
                    rows,
                    ..
                } => {
                    let bytes = element(row, &columns, *layout, *depth);
                    *rows = rows.saturating_add(bytes.unwrap_or(0));
                }
                Form::Lines(bytes) => {
                    // A row's line counts the bytes that text prints for it.
                    let line = lines::line(row).map_or(0, |line| line.len());
                    *bytes = bytes.saturating_add(line);
                }
                Form::Table { .. } => {
                    let fields = serde_json::to_value(row).unwrap_or(Value::Null);
                    let cells: Vec<Cow<'_, str>> =
                        columns.iter().map(|name| cell(fields.get(name))).collect();
                    self.form.add_line(&cells);
                }
                Form::Headed {
                    headers,
                    header_bytes,
                    content,
                    printed,
                } => {
                    *headers = value.key(1).is_some() || rows.len() > 1;
                    let position = value.row_indexes().get(index).copied().unwrap_or(index);
                    let header = if *headers {
                        header_len(value.key(position), position)
                    } else {
                        0
                    };
                    let bytes = match bytes_of.and_then(|bytes_of| bytes_of(value, index)) {
                        Some(bytes) => bytes.len(),
                        None => serde_json::to_vec(row).map_or(0, |line| line.len() + 1),
                    };
                    // `finish` leaves out a row too large for the budget on
                    // its own and prints the rows after it.
                    if header + bytes > budget && !(index == 0 && first_always) {
                        continue;
                    }
                    *header_bytes += header;
                    *content += bytes;
                    *printed += 1;
                }
            }
        }
        self.measured = rows.len();
        self.printed() < budget
    }

    /// Returns the bytes that `finish` prints for the rows that
    /// [`Room::has_room`] measured, when the batch has no failure.
    #[must_use]
    pub fn printed(&self) -> usize {
        match &self.form {
            Form::Json {
                layout,
                depth,
                empty,
                rows,
            } => empty.saturating_add(layout::array_len(self.measured, *rows, *layout, *depth)),
            Form::Lines(bytes) => *bytes,
            // A batch without rows prints no table.
            Form::Table { .. } if self.measured == 0 => 0,
            Form::Table {
                widths,
                padded,
                text,
            } => widths
                .iter()
                .zip(padded)
                .map(|(width, lines)| (width + 2) * lines)
                .sum::<usize>()
                .saturating_add(*text),
            Form::Headed {
                headers,
                header_bytes,
                content,
                printed,
            } => {
                let headers = if *headers { *header_bytes } else { 0 };
                headers + content + printed.saturating_sub(1)
            }
        }
    }
}

impl Form {
    /// Adds one line of the table, whose cells are `cells`. A line ends with
    /// its last non-empty cell, and each cell before that one is padded to
    /// its column's width and followed by two spaces.
    fn add_line(&mut self, cells: &[Cow<'_, str>]) {
        let Self::Table {
            widths,
            padded,
            text,
        } = self
        else {
            return;
        };
        let last = cells.iter().rposition(|cell| !cell.is_empty());
        // The padding of a cell counts characters, and its text counts
        // bytes.
        let mut bytes = 1;
        for (column, cell) in cells.iter().enumerate() {
            let chars = cell.chars().count();
            if let Some(width) = widths.get_mut(column) {
                *width = (*width).max(chars);
            }
            match last {
                Some(last) if column < last => {
                    bytes += cell.len() - chars;
                    if let Some(lines) = padded.get_mut(column) {
                        *lines += 1;
                    }
                }
                Some(last) if column == last => bytes += cell.len(),
                _ => {}
            }
        }
        *text += bytes;
    }
}

/// Returns the bytes of the JSON document of `document` without rows, as
/// `finish` prints it for a batch without rows or failures.
fn empty_document<T>(
    document: Document<T>,
    dry_run: bool,
    layout: JsonLayout,
) -> io::Result<usize> {
    let rows: [(); 0] = [];
    budget::measure(|counter: &mut Counter| {
        if !document.get_envelope().wraps() {
            return layout::write(counter, &rows, layout);
        }
        let extras = Extras {
            failures: Some(Failures::of(&[])),
            ..Extras::none()
        };
        write_document(counter, document, dry_run, layout, &rows, &extras)
    })
}

/// Returns the bytes of `row` as an element of the document's array of
/// rows, `depth` levels deep: its `columns`, or every field when there are
/// none, in `layout`.
fn element<R: Serialize>(
    row: &R,
    columns: &[&str],
    layout: JsonLayout,
    depth: usize,
) -> io::Result<usize> {
    if columns.is_empty() {
        return layout::element_len(row, layout, depth);
    }
    let value = serde_json::to_value(row)?;
    let selected = Selected {
        row: &value,
        columns,
    };
    layout::element_len(&selected, layout, depth)
}

/// Returns the bytes of the header line of the row at `position`, whose key
/// is `key`, as a headed batch prints it.
fn header_len(key: Option<&str>, position: usize) -> usize {
    result::headed::header(key, position).len()
}
