//! How a batch whose document asks for input order prints with `--json`:
//! one array of its rows and its failure documents in the order of their
//! keys (ADR-0016).

use std::io::{self, Write};

use serde::ser::SerializeSeq as _;
use serde::{Serialize, Serializer};

use super::Prepared;
use crate::Rows;
use crate::output::envelope::{Failures, Selected};
use crate::output::failure::PreparedFailure;
use crate::output::layout;

impl<T: Rows> Prepared<'_, T> {
    /// Writes the batch's first `shown` rows and every failure of
    /// `failures` as one array, each row and each of the batch's own
    /// failures at the position of its key, and the failure of the answers
    /// that the output budget left out last.
    ///
    /// A row prints as [`Prepared::write_unwrapped`] prints it. A failure
    /// prints as its failure document, which is the form's document when
    /// the guide has a failure form, so a reader tells a failure from a row
    /// by its `schema`.
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
        layout::write(out, &in_order, self.layout)
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
        let mut listed = self
            .failures
            .listed
            .iter()
            .zip(value.failure_indexes())
            .peekable();
        let mut seq = serializer.serialize_seq(None)?;
        for (number, row) in rows.iter().take(self.shown).enumerate() {
            // A row without an index, which only a result that is not a
            // batch has, prints after every failure.
            let at = row_indexes.get(number).copied().unwrap_or(usize::MAX);
            while let Some((failure, _)) = listed.next_if(|&(_, &index)| index < at) {
                seq.serialize_element(&Document(failure, self.failures))?;
            }
            match prepared.values.get(number).filter(|_| prepared.fields) {
                Some(selected) => seq.serialize_element(&Selected {
                    row: selected,
                    columns: prepared.columns,
                })?,
                None => seq.serialize_element(row)?,
            }
        }
        for (failure, _) in listed {
            seq.serialize_element(&Document(failure, self.failures))?;
        }
        if let Some(over) = self.failures.over {
            seq.serialize_element(&Document(over, self.failures))?;
        }
        seq.end()
    }
}

/// A failure document in the array: the form's document when the guide has
/// a failure form, and crumbkit's `crumbkit/failure/v1` document otherwise.
struct Document<'a>(&'a PreparedFailure<'a>, Failures<'a>);

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.1.form {
            Some(form) => form.json(self.0).serialize(serializer),
            None => self.0.serialize(serializer),
        }
    }
}
