//! The envelope that a document prints its result in with `--json`
//! (ADR-0071).

use super::{Document, is_one_line};

/// The fields that crumbkit writes beside the result in an envelope. A
/// declared key cannot take these names, because one object would then hold
/// a key twice.
pub(super) const RESERVED: [&str; 6] = [
    "dry_run",
    "total",
    "truncated",
    "summary",
    "cut",
    "failures",
];

/// The envelope that `crumbkit::finish` prints a document's result in with
/// `--json`, which a tool chooses with [`Document::envelope`] (ADR-0071).
///
/// An envelope without wrapping is an opt-out from crumbkit's conventions,
/// and the other envelopes are declarations. crumbkit-test's
/// `check_document_ids` fails a leaf whose document has no envelope unless
/// the tool opts out of the envelope rule for the command with a stated
/// reason (ADR-0036). crumbkit-test's `check_opt_outs` lists declared keys
/// as `envelope-keys` and failures in input order as `failures-in-order`.
///
/// The two in-order envelopes apply to a batch alone. crumbkit-test's
/// `check_guide` fails a leaf whose document is not a batch and has one of
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Envelope {
    /// crumbkit's envelope, with the id under `schema`, the result under
    /// `data` and a batch's failures under `failures`. A new document has
    /// it.
    Crumbkit,
    /// An envelope with the tool's own keys: `type_key` holds the
    /// document's id in place of `schema`, and `data_key` holds the result
    /// in place of `data`, such as `{"$type": "...", "entries": [...],
    /// "total": 12, "truncated": false}`.
    ///
    /// crumbkit still writes `dry_run`, `total`, `truncated`, `summary`,
    /// `cut` and `failures` under their own names, and every rule of the
    /// envelope holds: `--json` prints the compact columns of each row,
    /// `--long` adds the long columns, `--full` prints every field and the
    /// output budget marks a cut result with `cut`. `schemas list` shows
    /// the document with the kind `own-envelope` (ADR-0059).
    Keys {
        /// The key that holds the document's id.
        type_key: &'static str,
        /// The key that holds the result.
        data_key: &'static str,
    },
    /// The envelope of [`Envelope::Keys`] for a batch whose data key holds
    /// one array of each row and each failure document in the order of the
    /// keys, with the failure of the answers that the output budget left
    /// out last. The document has no `failures` field. Each failure
    /// document holds `key` and `index`, the failure's key and its position
    /// among the keys from 0, so a reader tells a failure from a row by its
    /// type key and matches it to its key (ADR-0016).
    KeysInOrder {
        /// The key that holds the document's id.
        type_key: &'static str,
        /// The key that holds the result.
        data_key: &'static str,
    },
    /// No envelope. `--json` prints the result alone, as its type
    /// serializes it with every field, without `schema`, `total` or
    /// `failures`, so a command can print an existing document unchanged
    /// while a tool migrates. `--full` is refused, because `--json` already
    /// prints every field. A batch prints its rows alone and its failures
    /// as text on standard error.
    Without,
    /// No envelope, for a batch that prints one array of each row and each
    /// failure document in the order of the keys, as
    /// [`Envelope::KeysInOrder`] does.
    WithoutInOrder,
}

impl Envelope {
    /// Returns the key that holds the document's id: the declared type key
    /// of [`Envelope::Keys`] and [`Envelope::KeysInOrder`], and `schema`
    /// for every other envelope.
    #[must_use]
    pub const fn type_key(self) -> &'static str {
        match self {
            Self::Keys { type_key, .. } | Self::KeysInOrder { type_key, .. } => type_key,
            Self::Crumbkit | Self::Without | Self::WithoutInOrder => "schema",
        }
    }

    /// Returns the key that holds the result: the declared data key of
    /// [`Envelope::Keys`] and [`Envelope::KeysInOrder`], and `data` for
    /// every other envelope.
    #[must_use]
    pub const fn data_key(self) -> &'static str {
        match self {
            Self::Keys { data_key, .. } | Self::KeysInOrder { data_key, .. } => data_key,
            Self::Crumbkit | Self::Without | Self::WithoutInOrder => "data",
        }
    }

    /// Reports whether the envelope declares its own keys.
    #[must_use]
    pub const fn has_own_keys(self) -> bool {
        matches!(self, Self::Keys { .. } | Self::KeysInOrder { .. })
    }

    /// Reports whether `--json` prints the result inside an envelope, which
    /// it does for every envelope but [`Envelope::Without`] and
    /// [`Envelope::WithoutInOrder`].
    #[must_use]
    pub const fn wraps(self) -> bool {
        !matches!(self, Self::Without | Self::WithoutInOrder)
    }

    /// Reports whether a batch prints its failures among its rows in input
    /// order, which it does for [`Envelope::KeysInOrder`] and
    /// [`Envelope::WithoutInOrder`].
    #[must_use]
    pub const fn failures_in_order(self) -> bool {
        matches!(self, Self::KeysInOrder { .. } | Self::WithoutInOrder)
    }
}

impl<T> Document<T> {
    /// Returns the same document printed in `envelope` with `--json`.
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Entry { id: u32 }
    /// # impl crumbkit_core::Row for Entry { const COLUMNS: &'static [&'static str] = &["id"]; }
    /// use crumbkit_core::{Document, Envelope, Listing};
    ///
    /// const ENTRIES: Document<Listing<Entry>> =
    ///     Document::new("tool", "get-entries", 1, "The entries.").envelope(Envelope::Keys {
    ///         type_key: "$type",
    ///         data_key: "entries",
    ///     });
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when the envelope declares keys and a key is not one
    /// non-empty line, the two keys are the same, or a key is one of the
    /// fields that crumbkit writes beside the result. A `const` that calls
    /// it with such keys fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit_core::{Document, Envelope};
    ///
    /// const ROWS: Document<()> = Document::new("tool", "get", 1, "Rows.")
    ///     .envelope(Envelope::Keys { type_key: "$type", data_key: "total" });
    /// ```
    ///
    /// ```compile_fail
    /// use crumbkit_core::{Document, Envelope};
    ///
    /// const ROWS: Document<()> = Document::new("tool", "get", 1, "Rows.")
    ///     .envelope(Envelope::Keys { type_key: "rows", data_key: "rows" });
    /// ```
    ///
    /// ```compile_fail
    /// use crumbkit_core::{Document, Envelope};
    ///
    /// const ROWS: Document<()> = Document::new("tool", "get", 1, "Rows.")
    ///     .envelope(Envelope::KeysInOrder { type_key: "", data_key: "rows" });
    /// ```
    #[must_use]
    pub const fn envelope(mut self, envelope: Envelope) -> Self {
        if let Envelope::Keys { type_key, data_key }
        | Envelope::KeysInOrder { type_key, data_key } = envelope
        {
            assert!(
                is_one_line(type_key) && is_one_line(data_key),
                "an envelope key is one non-empty line"
            );
            assert!(
                !same(type_key, data_key),
                "an envelope's type key and data key must differ; choose keys where the two keys differ"
            );
            assert!(
                !is_reserved(type_key) && !is_reserved(data_key),
                "an envelope key cannot be dry_run, total, truncated, summary, cut or failures, which crumbkit writes beside the result"
            );
        }
        self.envelope = envelope;
        self
    }

    /// Returns the envelope that [`Document::envelope`] chose, which is
    /// [`Envelope::Crumbkit`] unless the tool chose another.
    #[doc(hidden)]
    #[must_use]
    pub const fn get_envelope(&self) -> Envelope {
        self.envelope
    }
}

/// Returns whether `name` is one of the fields that crumbkit writes beside
/// the result.
const fn is_reserved(name: &str) -> bool {
    let mut index = 0;
    // A `const fn` cannot call iterator methods, so this loop walks the
    // names by index.
    while index < RESERVED.len() {
        if same(name, RESERVED[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// Returns whether `a` and `b` hold the same bytes. `str`'s `==` is not a
/// `const fn`.
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}
