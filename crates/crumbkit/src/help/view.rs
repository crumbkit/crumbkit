//! The view flags `--fields`, `--long` and `--full`, and the rule that
//! decides from a command's document which of them the command offers
//! (ADR-0016).

use crate::document::ListingText;
use crate::refusal::Refusal;
use crate::{Document, Row, Shape};

#[cfg(test)]
mod tests;

/// One of the output flags that shape a table: `--fields`, `--long` and
/// `--full`.
///
/// A leaf with a document offers each view flag that the document can use,
/// as [`Leaf::offers`](super::Leaf::offers) reports. `parse` leaves the
/// other view flags out of the leaf's help and refuses them when the caller
/// types them, and `finish` and `stream` refuse them by the same rule.
/// `--long` and `--full` with `--json` on a command whose JSON already holds
/// the whole result add nothing to it, and are accepted with a note in place
/// of the refusal, as [`Leaf::adds_nothing`](super::Leaf::adds_nothing)
/// describes (ADR-0149).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFlag {
    /// `--fields`, which selects the columns of a table.
    Fields,
    /// `--long`, which adds the row type's long columns.
    Long,
    /// `--full`, which prints every field of every row in JSON.
    Full,
}

impl ViewFlag {
    /// Every view flag, in the order in which crumbkit checks them.
    pub(crate) const ALL: [Self; 3] = [Self::Fields, Self::Long, Self::Full];

    /// Returns the flag as a caller types it, such as `--long`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fields => "--fields",
            Self::Long => "--long",
            Self::Full => "--full",
        }
    }

    /// Returns the id of the flag's argument in [`Output`](crate::Output),
    /// such as `long`.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::Fields => "fields",
            Self::Long => "long",
            Self::Full => "full",
        }
    }

    /// Returns the view flag whose argument id in [`Output`](crate::Output)
    /// is `id`, or `None` when `id` names no view flag.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|flag| flag.id() == id)
    }
}

/// Why a command does not offer a view flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unoffered {
    /// The command prints prose, which has no columns.
    Prose,
    /// The command prints bytes, which have no columns.
    Bytes,
    /// The command's row type has no long columns, so `--long` adds none.
    NoLongColumns,
    /// The command's document has no envelope, or is headed, or its row
    /// type sets `Row::WHOLE`, so `--json` already prints every field and
    /// `--full` adds nothing.
    Unwrapped,
    /// The command's document is headed, so its text prints each answer's
    /// bytes, which have no columns, and its JSON already holds every field.
    Headed,
    /// The command's row type sets `Row::WHOLE` and names no columns, so its
    /// text prints each row's own line and has no table to shape
    /// (ADR-0065, ADR-0072).
    NoColumns,
    /// The command prints no result on standard output, as
    /// [`Leaf::prints_no_result`](super::Leaf::prints_no_result) declares
    /// (ADR-0143).
    NoResult,
}

/// Whether a command prints one document or a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// One document whose result has the shape, which
    /// [`Leaf::document`](super::Leaf::document) sets.
    Document(Shape),
    /// A stream of rows, one line for each, which
    /// [`Leaf::stream`](super::Leaf::stream) sets.
    Stream,
}

/// What a command prints, as far as the view flags depend on it. A leaf
/// builds it from its declared document, and `finish` and `stream` build it
/// from the row type and the document they print.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Printed {
    /// The shape of the document's result, or a stream.
    pub(crate) kind: Kind,
    /// The row type's `Row::COLUMNS`.
    pub(crate) columns: &'static [&'static str],
    /// The row type's `Row::LONG_COLUMNS`.
    pub(crate) long: &'static [&'static str],
    /// Whether the document has crumbkit's envelope.
    pub(crate) envelope: bool,
    /// Whether the document is headed.
    pub(crate) headed: bool,
    /// Whether the row type sets `Row::WHOLE`.
    pub(crate) whole: bool,
    /// What the text of a listing does beyond its columns.
    pub(crate) listing: ListingText,
}

impl Printed {
    /// Returns what a command prints when its rows are `R`, printed as
    /// `document` with `kind`.
    pub(crate) fn of<R: Row, T>(kind: Kind, document: &Document<T>) -> Self {
        Self {
            kind,
            columns: R::COLUMNS,
            long: R::LONG_COLUMNS,
            envelope: document.get_envelope().wraps(),
            headed: document.is_headed(),
            whole: R::WHOLE,
            listing: document.listing_text(),
        }
    }
}

/// Returns why a command that prints `printed` does not offer `flag`, or
/// `None` when it offers it.
///
/// A prose or bytes result offers no view flag. Rows offer `--fields`.
/// Rows offer `--long` when the row type has long columns, the document
/// lists only the rows that report a problem (ADR-0138) or the document
/// declares a long note (ADR-0139). A headed document offers no `--long`.
/// Rows offer `--full` when the document has the envelope, is not headed
/// and its rows do not print whole (ADR-0062). Rows that print whole and
/// have no columns offer neither `--fields` nor `--long` (ADR-0065).
pub(crate) const fn unoffered(printed: &Printed, flag: ViewFlag) -> Option<Unoffered> {
    match printed.kind {
        Kind::Document(Shape::Prose) => return Some(Unoffered::Prose),
        Kind::Document(Shape::Bytes) => return Some(Unoffered::Bytes),
        Kind::Document(_) | Kind::Stream => {}
    }
    let no_columns = printed.whole && printed.columns.is_empty() && printed.long.is_empty();
    match flag {
        ViewFlag::Fields | ViewFlag::Long if no_columns => Some(Unoffered::NoColumns),
        ViewFlag::Long if printed.headed => Some(Unoffered::Headed),
        ViewFlag::Long if printed.long.is_empty() && !printed.listing.changed_by_long() => {
            Some(Unoffered::NoLongColumns)
        }
        ViewFlag::Full if !printed.envelope || printed.headed || printed.whole => {
            Some(Unoffered::Unwrapped)
        }
        _ => None,
    }
}

/// Returns the refusal of `flag` on a command that does not offer it for
/// `reason`. The refusal names the flag under `fields`, and it prints as a
/// refused command line does, with the code `<tool>::cli::usage` and exit
/// code 2, or the guide's rename and exit for them.
pub(crate) fn refusal(flag: ViewFlag, reason: Unoffered) -> Refusal {
    let does = match flag {
        ViewFlag::Fields => "--fields selects the columns of a table",
        ViewFlag::Long => "--long adds the long columns of a table",
        ViewFlag::Full => "--full prints every field of every row",
    };
    let hint = match (reason, flag) {
        (Unoffered::NoResult, ViewFlag::Fields) => "drop --fields",
        (Unoffered::NoResult, ViewFlag::Long) => "drop --long",
        (Unoffered::NoResult, ViewFlag::Full) => "drop --full",
        (Unoffered::Prose | Unoffered::Bytes, ViewFlag::Full) | (Unoffered::Unwrapped, _) => {
            "drop --full, because --json already prints the whole result"
        }
        (Unoffered::Prose, _) => "use --json to read every field of the result",
        (Unoffered::Bytes, _) => "use --json to read the bytes as text or base64",
        (Unoffered::NoLongColumns, _) => "use --fields to name the columns to show",
        (Unoffered::Headed, _) => "use --json to read every field of each answer",
        (Unoffered::NoColumns, _) => "use --json to read every field of each row",
    };
    Refusal::flag_value(
        format!("{does}, and {}", because(reason)),
        hint.to_owned(),
        flag.name(),
        None,
    )
}

/// Returns the clause of a refusal that says why the command does not
/// offer a view flag for `reason`.
const fn because(reason: Unoffered) -> &'static str {
    match reason {
        Unoffered::Prose => "this command prints prose",
        Unoffered::Bytes => "this command prints raw bytes",
        Unoffered::NoLongColumns => "this command's rows have none",
        Unoffered::Unwrapped => "this command's JSON already holds every field",
        Unoffered::Headed => "this command's text prints each answer's bytes",
        Unoffered::NoColumns => "this command's rows have no columns",
        Unoffered::NoResult => "this command prints no result",
    }
}

/// Reports whether `flag`, passed with `--json` when `json` is true, adds
/// nothing to the JSON of a command that prints `printed` (ADR-0149).
///
/// `--long` and `--full` add fields to the JSON, so they add nothing when
/// the JSON already holds the whole result: for prose and bytes, for a
/// document or stream without the envelope, for a headed document and for
/// rows whose type sets `Row::WHOLE`. crumbkit accepts such a flag, prints
/// what `--json` alone prints and prints the note of [`adds_nothing_note`],
/// where it would otherwise refuse the flag as unoffered. `--fields`
/// selects fields, so it never adds nothing, and without `--json` the text
/// still uses `--long` or refuses it.
pub(crate) const fn adds_nothing(printed: &Printed, flag: ViewFlag, json: bool) -> bool {
    let whole_result = matches!(printed.kind, Kind::Document(Shape::Prose | Shape::Bytes))
        || !printed.envelope
        || printed.headed
        || printed.whole;
    json && whole_result && matches!(flag, ViewFlag::Long | ViewFlag::Full)
}

/// Returns the note that says `flag` adds nothing to the JSON, such as
/// `note: --long adds nothing to --json here; the JSON already holds the
/// whole result.`, for a flag that [`adds_nothing`] accepts.
pub(crate) fn adds_nothing_note(flag: ViewFlag) -> String {
    format!(
        "note: {} adds nothing to --json here; the JSON already holds the whole result.",
        flag.name()
    )
}

/// Returns the refusal of `--fields` without `--json` on a command whose
/// document is headed, or `None` when `json` is on or `fields` is off
/// (ADR-0062). `fields` says whether `--fields` names at least one field,
/// and `json` whether the caller passed `--json`.
///
/// The text of a headed document prints each answer's bytes, which have no
/// columns, so `--fields` selects only the fields of its JSON. The refusal
/// names `--fields` under `argument`, and it prints as a refused command
/// line does.
pub(crate) fn headed(fields: bool, json: bool) -> Option<Refusal> {
    (fields && !json).then(|| {
        Refusal::flag_value(
            "--fields selects the fields of JSON rows, and this command's text prints each answer's bytes".to_owned(),
            "add --json, or drop --fields".to_owned(),
            ViewFlag::Fields.name(),
            None,
        )
    })
}
