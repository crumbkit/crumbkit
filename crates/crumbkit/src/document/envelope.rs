//! The envelope that a document prints its result in with `--json`
//! (ADR-0071).

use super::Document;

/// The envelope that `crumbkit::finish` prints a document's result in with
/// `--json`, which a tool chooses with [`Document::envelope`] (ADR-0071).
///
/// An envelope without wrapping is an opt-out from crumbkit's conventions.
/// crumbkit-test's `check_document_ids` fails a leaf whose document has no
/// envelope unless the tool opts out of the envelope rule for the command
/// with a stated reason (ADR-0036).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Envelope {
    /// crumbkit's envelope, with the id under `schema`, the result under
    /// `data` and a batch's failures under `failures`. A new document has
    /// it.
    Crumbkit,
    /// No envelope. `--json` prints the result alone, as its type
    /// serializes it with every field, without `schema`, `total` or
    /// `failures`, so a command can print an existing document unchanged
    /// while a tool migrates. `--full` is refused, because `--json` already
    /// prints every field. A batch prints its rows alone and its failures
    /// as text on standard error.
    Without,
}

impl Envelope {
    /// Reports whether `--json` prints the result inside an envelope, which
    /// it does for [`Envelope::Crumbkit`].
    #[must_use]
    pub const fn wraps(self) -> bool {
        !matches!(self, Self::Without)
    }
}

impl<T> Document<T> {
    /// Returns the same document printed in `envelope` with `--json`.
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Entry { id: u32 }
    /// # impl crumbkit::Row for Entry { const COLUMNS: &'static [&'static str] = &["id"]; }
    /// use crumbkit::{Document, Envelope, Listing};
    ///
    /// const ENTRIES: Document<Listing<Entry>> =
    ///     Document::new("tool", "get-entries", 1, "The entries.").envelope(Envelope::Without);
    /// ```
    #[must_use]
    pub const fn envelope(mut self, envelope: Envelope) -> Self {
        self.envelope = envelope;
        self
    }

    /// Returns the envelope that [`Document::envelope`] chose, which is
    /// [`Envelope::Crumbkit`] unless the tool chose another. It is hidden
    /// because only crumbkit and crumbkit-test read it.
    #[doc(hidden)]
    #[must_use]
    pub const fn get_envelope(&self) -> Envelope {
        self.envelope
    }
}
