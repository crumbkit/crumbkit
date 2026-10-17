//! How a batch whose document asks for input order prints with `--json`:
//! one array of its rows and its failure documents in the order of their
//! keys, alone or under the data key of a document with its own envelope
//! keys (ADR-0016, ADR-0059).

use std::io::{self, Write};

use serde::ser::SerializeSeq as _;
use serde::{Serialize, Serializer};

use super::Prepared;
use crate::Rows;
use crate::form::FailureForm;
use crate::output::envelope::{Extras, Failures, Selected};
use crate::output::failure::PreparedFailure;
use crate::output::layout;

impl<T: Rows> Prepared<'_, T> {
    /// Writes the batch's first `shown` rows and every failure of
    /// `failures` as one array, each row, each of the batch's own failures
    /// and the failure of each key left out at the position of its key, and
    /// the failure that names the answers that the output budget left out
    /// last.
    ///
    /// A document with its own envelope keys holds the array under its data
    /// key, and a document without the envelope prints the array alone.
    ///
    /// A row prints with the columns that the view flags choose, which
    /// leave every field of a row in a document without the envelope
    /// unless the caller passed `--fields`. A failure prints as its failure
    /// document, which is the form's document when the guide has a failure
    /// form, so a reader tells a failure from a row by its `schema`.
    pub(super) fn write_in_order(
        &self,
        out: &mut impl Write,
        shown: usize,
        failures: Failures<'_>,
    ) -> io::Result<()> {
        let in_order = InOrder {
            prepared: self,
            shown,
            failures,
        };
        if self.document.has_envelope() {
            self.envelope(out, &in_order, &Extras::none())
        } else {
            layout::write(out, &in_order, self.style)
        }
    }
}

/// The array that [`Prepared::write_in_order`] writes, serialized one
/// element at a time so that nothing is collected first.
struct InOrder<'p, 'a, T> {
    prepared: &'p Prepared<'a, T>,
    shown: usize,
    failures: Failures<'p>,
}

impl<T: Rows> Serialize for InOrder<'_, '_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let prepared = self.prepared;
        let value = prepared.value;
        let rows = value.rows();
        let row_indexes = value.row_indexes();
        let mut failures = self.failures.positioned().peekable();
        let mut seq = serializer.serialize_seq(None)?;
        for (number, row) in rows.iter().take(self.shown).enumerate() {
            // A row without an index, which only a result that is not a
            // batch has, prints after every failure with an index.
            let at = row_indexes.get(number).copied().unwrap_or(usize::MAX);
            while let Some((_, failure)) = failures.next_if(|&(index, _)| index < at) {
                seq.serialize_element(&InOrderFailure(failure, self.failures.form))?;
            }
            // The rows are serialized only when the view flags select
            // columns, so a row without a serialized value prints whole.
            match prepared.values.get(number) {
                Some(selected) => seq.serialize_element(&Selected {
                    row: selected,
                    columns: prepared.columns,
                })?,
                None => seq.serialize_element(row)?,
            }
        }
        // The failure that names the answers left out sorts last.
        for (_, failure) in failures {
            seq.serialize_element(&InOrderFailure(failure, self.failures.form))?;
        }
        seq.end()
    }
}

/// A failure document in the array: the document of the guide's failure
/// form when it has one, and crumbkit's `crumbkit/failure/v1` document
/// otherwise.
pub(super) struct InOrderFailure<'a>(
    pub(super) &'a PreparedFailure<'a>,
    pub(super) Option<&'static dyn FailureForm>,
);

impl Serialize for InOrderFailure<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.1 {
            Some(form) => form.json(self.0).serialize(serializer),
            None => self.0.serialize(serializer),
        }
    }
}
