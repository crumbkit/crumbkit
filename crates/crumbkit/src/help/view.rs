//! The view flags `--fields`, `--long` and `--full`, and the rule that
//! decides from a command's document which of them the command offers
//! (ADR-0016).

use crate::Shape;
use crate::refusal::Refusal;

/// One of the output flags that shape a table: `--fields`, `--long` and
/// `--full`.
///
/// A leaf with a document offers each view flag that the document can use,
/// as [`Leaf::offers`](super::Leaf::offers) reports. `parse` leaves the
/// other view flags out of the leaf's help and refuses them when the caller
/// types them, and `finish` and `stream` refuse them by the same rule.
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
}

/// Returns why a command does not offer `flag`, or `None` when it offers
/// it. The command prints a result of `shape`, or a stream of rows when
/// `shape` is `None`. Its row type's `Row::COLUMNS` are `columns` and its
/// `Row::LONG_COLUMNS` are `long`. Its document has crumbkit's envelope
/// when `envelope` is set and is headed when `headed` is set, and its row
/// type sets `Row::WHOLE` when `whole` is set.
///
/// A prose or bytes result offers no view flag. Rows offer `--fields`,
/// offer `--long` when the row type has long columns and the document is
/// not headed, and offer `--full` when the document has the envelope, is
/// not headed and its rows do not print whole (ADR-0062). Rows that print
/// whole and have no columns offer neither `--fields` nor `--long`
/// (ADR-0065).
pub(crate) const fn unoffered(
    shape: Option<Shape>,
    columns: &[&str],
    long: &[&str],
    envelope: bool,
    headed: bool,
    whole: bool,
    flag: ViewFlag,
) -> Option<Unoffered> {
    match shape {
        Some(Shape::Prose) => return Some(Unoffered::Prose),
        Some(Shape::Bytes) => return Some(Unoffered::Bytes),
        _ => {}
    }
    let no_columns = whole && columns.is_empty() && long.is_empty();
    match flag {
        ViewFlag::Fields | ViewFlag::Long if no_columns => Some(Unoffered::NoColumns),
        ViewFlag::Long if headed => Some(Unoffered::Headed),
        ViewFlag::Long if long.is_empty() => Some(Unoffered::NoLongColumns),
        ViewFlag::Full if !envelope || headed || whole => Some(Unoffered::Unwrapped),
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
    }
}

/// The view flags that the caller typed, each set when the flag is on the
/// command line, and `fields` set when `--fields` names at least one field.
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one independent view flag, and no two of them form a state machine"
)]
pub(crate) struct Typed {
    pub(crate) json: bool,
    pub(crate) fields: bool,
    pub(crate) long: bool,
    pub(crate) full: bool,
}

/// Returns the refusal of `--fields` without `--json` on a command whose
/// document is headed, or `None` when `typed` holds `--json` or no
/// `--fields` (ADR-0062).
///
/// The text of a headed document prints each answer's bytes, which have no
/// columns, so `--fields` selects only the fields of its JSON. The refusal
/// names `--fields` under `argument`, and it prints as a refused command
/// line does.
pub(crate) fn headed(typed: Typed) -> Option<Refusal> {
    (typed.fields && !typed.json).then(|| {
        Refusal::flag_value(
            "--fields selects the fields of JSON rows, and this command's text prints each answer's bytes".to_owned(),
            "add --json, or drop --fields".to_owned(),
            ViewFlag::Fields.name(),
            None,
        )
    })
}

/// Returns the refusal of the view flags in `typed` when two of them cannot
/// work together, or `None` when they can.
///
/// `--fields` prints only the fields it names, so it cannot be used with
/// `--long` or `--full`, and `--full` prints every field in JSON, so it needs
/// `--json`. The refusal names the flag to drop or the one that needs
/// `--json` under `argument`, and it prints as a refused command line does.
pub(crate) fn pairing(typed: Typed) -> Option<Refusal> {
    let Typed {
        json,
        fields,
        long,
        full,
    } = typed;
    let (message, hint, flag) = if fields && long {
        (
            "--fields prints only the fields it names, so it cannot be used with --long",
            "drop --long, and name the long columns in --fields",
            ViewFlag::Long,
        )
    } else if fields && full {
        (
            "--fields prints only the fields it names, so it cannot be used with --full",
            "drop --full to print only the named fields, or drop --fields to print every field",
            ViewFlag::Full,
        )
    } else if full && !json {
        (
            "--full prints every field of every row in JSON, so it needs --json",
            "add --json, or drop --full",
            ViewFlag::Full,
        )
    } else {
        return None;
    };
    Some(Refusal::flag_value(
        message.to_owned(),
        hint.to_owned(),
        flag.name(),
        None,
    ))
}
