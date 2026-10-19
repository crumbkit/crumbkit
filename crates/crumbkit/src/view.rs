//! The view that a run's four view flags select, parsed once from
//! [`ViewFlags`] into a [`View`] that every printer reads.
//!
//! The flags `--json`, `--long`, `--full` and `--fields` select how a
//! run's answer prints. Three pairs of them cannot work together:
//! `--fields` with `--long`, `--fields` with `--full`, and `--full`
//! without `--json`. [`View::try_from`] refuses those pairs, so a
//! [`View`] holds only a combination that every printer accepts.

use crate::ViewFlags;
use crate::help::view::ViewFlag;
use crate::refusal::Refusal;

/// The view that a run's flags select: a text table or JSON rows with a
/// selection of fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum View {
    /// The answer prints as text, with the table's columns.
    Text(Table),
    /// The answer prints as JSON, with the selected fields of each row.
    Json(Selection),
}

/// The columns of a text table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Table {
    /// The row's compact columns, the default.
    Compact,
    /// The row's long columns, from `--long`.
    Long,
    /// The columns that `--fields` names, in its order.
    Named(Vec<String>),
}

/// The fields of each JSON row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    /// The row's compact columns, the default.
    Columns,
    /// The row's long columns, from `--long`.
    Long,
    /// Every field of every row, from `--full`.
    Full,
    /// The fields that `--fields` names, in its order.
    Named(Vec<String>),
}

impl View {
    /// Returns whether the answer prints as JSON.
    pub(crate) const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns whether `--long` selected the long columns.
    pub(crate) const fn is_long(&self) -> bool {
        matches!(self, Self::Text(Table::Long) | Self::Json(Selection::Long))
    }

    /// Returns whether `--full` selected every field.
    pub(crate) const fn is_full(&self) -> bool {
        matches!(self, Self::Json(Selection::Full))
    }

    /// Returns the names that `--fields` gave, or `None` when the flag
    /// was absent.
    pub(crate) fn names(&self) -> Option<&[String]> {
        match self {
            Self::Text(Table::Named(names)) | Self::Json(Selection::Named(names)) => {
                Some(names.as_slice())
            }
            Self::Text(Table::Compact | Table::Long)
            | Self::Json(Selection::Columns | Selection::Long | Selection::Full) => None,
        }
    }

    /// Returns the names that `--fields` gave, or an empty slice when the
    /// flag was absent.
    pub(crate) fn fields(&self) -> &[String] {
        self.names().unwrap_or_default()
    }
}

impl TryFrom<ViewFlags> for View {
    type Error = Refusal;

    /// Builds the view that `flags` select, or returns the refusal of the
    /// first pair that cannot work together: `--fields` with `--long`,
    /// then `--fields` with `--full`, then `--full` without `--json`.
    fn try_from(flags: ViewFlags) -> Result<Self, Refusal> {
        let ViewFlags {
            json,
            fields,
            long,
            full,
        } = flags;
        let named = !fields.is_empty();
        if named && long {
            return Err(pairing(
                "--fields prints only the fields it names, so it cannot be used with --long",
                "drop --long, and name the long columns in --fields",
                ViewFlag::Long,
            ));
        }
        if named && full {
            return Err(pairing(
                "--fields prints only the fields it names, so it cannot be used with --full",
                "drop --full to print only the named fields, or drop --fields to print every field",
                ViewFlag::Full,
            ));
        }
        if full && !json {
            return Err(pairing(
                "--full prints every field of every row in JSON, so it needs --json",
                "add --json, or drop --full",
                ViewFlag::Full,
            ));
        }
        Ok(match (json, named, full, long) {
            (true, true, _, _) => Self::Json(Selection::Named(fields)),
            (true, false, true, _) => Self::Json(Selection::Full),
            (true, false, false, true) => Self::Json(Selection::Long),
            (true, false, false, false) => Self::Json(Selection::Columns),
            (false, true, _, _) => Self::Text(Table::Named(fields)),
            (false, false, _, true) => Self::Text(Table::Long),
            (false, false, _, false) => Self::Text(Table::Compact),
        })
    }
}

/// Returns the refusal of the value of `flag` that says `message` and
/// whose recovery is the hint `hint`.
fn pairing(message: &str, hint: &str, flag: ViewFlag) -> Refusal {
    Refusal::flag_value(message.to_owned(), hint.to_owned(), flag.name(), None)
}

#[cfg(test)]
mod tests;
