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
    /// The command's document has no envelope, so `--json` already prints
    /// every field and `--full` adds nothing.
    Unwrapped,
}

/// Returns why a command does not offer `flag`, or `None` when it offers
/// it. The command prints a result of `shape`, or a stream of rows when
/// `shape` is `None`. Its row type has long columns when `long_columns` is
/// set, and its document has crumbkit's envelope when `envelope` is set.
///
/// A prose or bytes result offers no view flag. Rows offer `--fields`,
/// offer `--long` when the row type has long columns, and offer `--full`
/// when the document has the envelope.
pub(crate) const fn unoffered(
    shape: Option<Shape>,
    long_columns: bool,
    envelope: bool,
    flag: ViewFlag,
) -> Option<Unoffered> {
    match shape {
        Some(Shape::Prose) => return Some(Unoffered::Prose),
        Some(Shape::Bytes) => return Some(Unoffered::Bytes),
        _ => {}
    }
    match flag {
        ViewFlag::Long if !long_columns => Some(Unoffered::NoLongColumns),
        ViewFlag::Full if !envelope => Some(Unoffered::Unwrapped),
        _ => None,
    }
}

/// Returns the refusal of `flag` on a command that does not offer it for
/// `reason`. The refusal names the flag under `fields`, and it prints as a
/// refused command line does, with the code `crumbkit::cli::usage` and exit
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
    }
}

/// Returns the refusal of the view flags that the caller typed, `--fields`
/// when `fields` is set and `--long`, `--full` and `--json` when the flag of
/// that name is set, when two of them cannot work together, or `None` when
/// they can.
///
/// `--fields` prints only the fields it names, so it cannot be used with
/// `--long` or `--full`, and `--full` prints every field in JSON, so it needs
/// `--json`. The refusal names the flag to drop or the one that needs
/// `--json` under `argument`, and it prints as a refused command line does.
pub(crate) fn pairing(json: bool, fields: bool, long: bool, full: bool) -> Option<Refusal> {
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
