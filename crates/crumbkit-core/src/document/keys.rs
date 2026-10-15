//! The keys of a document's envelope, which a tool whose readers know
//! another envelope declares in place of `schema` and `data` (ADR-0059).

use super::{Document, Envelope, is_one_line};
use crate::{Listing, Row};

/// The fields that crumbkit writes beside the result in an envelope. A
/// declared key and a field of a summary at the top cannot take these
/// names, because one object would then hold a key twice.
const RESERVED: [&str; 6] = [
    "dry_run",
    "total",
    "truncated",
    "summary",
    "cut",
    "failures",
];

impl<T> Document<T> {
    /// Returns the same document with the tool's own envelope keys:
    /// `type_key` holds the document's type in place of `schema`, and
    /// `data_key` holds the result in place of `data`, such as
    /// `{"$type": "...", "entries": [...], "total": 12, "truncated": false}`.
    ///
    /// crumbkit still writes `dry_run`, `total`, `truncated`, `summary`,
    /// `cut` and `failures` under their own names, and every rule of the
    /// envelope holds: `--json` prints the compact columns of each row,
    /// `--long` adds the long columns, `--full` prints every field and the
    /// output budget marks a cut result with `cut`. The type key holds the
    /// document's id unless the tool also calls [`Document::with_type`].
    ///
    /// It is a declaration, so it needs no opt-out, and crumbkit-test's
    /// `check_opt_outs` lists it under the customisations as
    /// `envelope-keys`. `schemas list` shows the document with the kind
    /// `own-envelope` (ADR-0059).
    ///
    /// ```
    /// # #[derive(serde::Serialize, schemars::JsonSchema)]
    /// # struct Entry { id: u32 }
    /// # impl crumbkit_core::Row for Entry { const COLUMNS: &'static [&'static str] = &["id"]; }
    /// use crumbkit_core::{Document, Listing};
    ///
    /// const ENTRIES: Document<Listing<Entry>> =
    ///     Document::new("tool", "get-entries", 1, "The entries.").with_keys("$type", "entries");
    ///
    /// assert_eq!(ENTRIES.data_key(), "entries");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless each key is one non-empty line, the two keys differ,
    /// neither is one of the fields that crumbkit writes beside the result,
    /// and the document has the envelope. A `const` that calls it with such
    /// keys fails to compile.
    #[must_use]
    pub const fn with_keys(mut self, type_key: &'static str, data_key: &'static str) -> Self {
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
        assert!(
            !matches!(self.envelope, Envelope::Without),
            "a document from without_envelope has no envelope keys; call with_keys or without_envelope, not both"
        );
        self.envelope = Envelope::Keys { type_key, data_key };
        self
    }

    /// Returns the same document whose type key holds `value` in place of
    /// the document's id, such as a type string that the tool's readers
    /// already match. `schemas list` and the schema's `title` still show
    /// the id, and the schema gives the type key `value` as its constant
    /// (ADR-0059).
    ///
    /// # Panics
    ///
    /// Panics unless the document declares its keys with
    /// [`Document::with_keys`] first and `value` is one non-empty line. A
    /// `const` that calls it otherwise fails to compile.
    #[must_use]
    pub const fn with_type(mut self, value: &'static str) -> Self {
        assert!(
            self.has_own_keys(),
            "a type value needs the document's own keys; call with_keys first"
        );
        assert!(is_one_line(value), "a type value is one non-empty line");
        self.type_value = Some(value);
        self
    }

    /// Returns the key that holds the document's type: `schema`, or the
    /// key that [`Document::with_keys`] declared.
    #[must_use]
    pub const fn type_key(&self) -> &'static str {
        match self.envelope {
            Envelope::Keys { type_key, .. } => type_key,
            Envelope::Crumbkit | Envelope::Without => "schema",
        }
    }

    /// Returns the key that holds the result: `data`, or the key that
    /// [`Document::with_keys`] declared.
    #[must_use]
    pub const fn data_key(&self) -> &'static str {
        match self.envelope {
            Envelope::Keys { data_key, .. } => data_key,
            Envelope::Crumbkit | Envelope::Without => "data",
        }
    }

    /// Returns the value that [`Document::with_type`] set, or `None` when
    /// the type key holds the document's id.
    #[must_use]
    pub const fn type_value(&self) -> Option<&'static str> {
        self.type_value
    }

    /// Reports whether [`Document::with_keys`] was called.
    #[must_use]
    pub const fn has_own_keys(&self) -> bool {
        matches!(self.envelope, Envelope::Keys { .. })
    }

    /// Reports whether [`Document::summary_at_top`] was called, so a
    /// listing's summary prints its fields at the top of the document.
    #[must_use]
    pub const fn prints_summary_at_top(&self) -> bool {
        self.summary_top
    }
}

impl<R: Row, S> Document<Listing<R, S>> {
    /// Returns the same listing document with the fields of its summary
    /// at the top of the document, beside the type key, the data key,
    /// `total` and `truncated`, in place of one object under `summary`. A
    /// tool whose listing document carries fields of its own, such as
    /// `location`, sets them as the listing's summary, and the summary's
    /// `Display` is the line that the text prints above the table.
    ///
    /// The summary must serialize as an object, and none of its fields may
    /// take the type key, the data key or a field that crumbkit writes,
    /// such as `total`. `crumbkit::finish` reports a summary that breaks
    /// either rule as a failed write, so the run exits with 1 and prints
    /// no document. The document's schema holds the summary's schema under
    /// `allOf` (ADR-0059).
    ///
    /// # Panics
    ///
    /// Panics unless the document declares its keys with
    /// [`Document::with_keys`], because crumbkit's own envelope keeps the
    /// summary under `summary`. A `const` that calls it on such a document
    /// fails to compile.
    #[must_use]
    pub const fn summary_at_top(mut self) -> Self {
        assert!(
            self.has_own_keys(),
            "a summary at the top needs a document without crumbkit's own envelope; call with_keys first"
        );
        self.summary_top = true;
        self
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
