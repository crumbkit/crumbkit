//! The lines that a leaf's `-h` and `--help` print about the rows of its
//! document: `Problems:`, `Rows:` and `Long note:`.

use super::Leaf;
use crate::help::ViewFlag;

/// The line that the `-h` and `--help` of a leaf set with
/// [`Leaf::stream`] print after the effects line, in help's wording
/// (ADR-0041). Another rendering of help, such as a Markdown reference,
/// prints this line so that it describes a stream as help does.
// crumbkit-markdown prints this line in a reference.
#[doc(hidden)]
pub const STREAM_LINE: &str =
    "Output: one row per line as each arrives, and --json prints one JSON document per line.";

/// The line that the `-h` and `--help` of a leaf whose document holds
/// [`Bytes`](crate::Bytes) print after the effects line, in help's wording.
// crumbkit-markdown prints this line in a reference.
#[doc(hidden)]
pub const BYTES_LINE: &str =
    "Output: the bytes unchanged, and --json prints them as text or base64.";

/// The line that the `-h` and `--help` of a leaf whose document is headed
/// print after the effects line, in help's wording (ADR-0062).
// crumbkit-markdown prints this line in a reference.
#[doc(hidden)]
pub const HEADED_LINE: &str = "Output: each answer's bytes unchanged, under a ==> KEY <== header when there are several keys, and --json prints the bytes as text or base64.";

impl Leaf {
    /// Returns the `Problems:` line that the leaf's `-h` and `--help` print
    /// after the `Output:` line when its row type names a problem field,
    /// such as `Problems: a value in the problem column marks a row with a
    /// problem, and the command then exits with 1.`, or `None` otherwise.
    pub(crate) fn problems_line(&self) -> Option<String> {
        let field = self.problem()?;
        let exit = crate::exit::problem_rows();
        Some(format!(
            "Problems: a value in the {field} column marks a row with a problem, and the command then exits with {exit}."
        ))
    }

    /// Returns the `Rows:` line that the leaf's `-h` and `--help` print
    /// after the `Problems:` line when its document lists only the rows
    /// that report a problem, such as `Rows: the default text lists only
    /// the rows that report a problem, and --long, --fields or --json lists
    /// every row.`, or `None` otherwise (ADR-0138, ADR-0140). The line
    /// names `--long` and `--fields` only when the command offers them.
    pub(crate) fn rows_line(&self) -> Option<String> {
        if !self.get_document()?.lists_only_problems() {
            return None;
        }
        let offered: Vec<&str> = [ViewFlag::Long, ViewFlag::Fields]
            .into_iter()
            .filter(|flag| self.offers(*flag))
            .map(ViewFlag::name)
            .collect();
        let flags = if offered.is_empty() {
            "--json".to_owned()
        } else {
            format!("{} or --json", offered.join(", "))
        };
        Some(format!(
            "Rows: the default text lists only the rows that report a problem, and {flags} lists every row."
        ))
    }

    /// Returns the `Long note:` line that the leaf's `-h` and `--help`
    /// print after the `Rows:` line when its document declares a long note,
    /// `Long note: --long prints a note on standard error.`, or `None`
    /// otherwise (ADR-0139). A leaf that does not offer `--long` has none.
    pub(crate) fn long_note_line(&self) -> Option<&'static str> {
        (self.get_document()?.has_long_note() && self.offers(ViewFlag::Long))
            .then_some("Long note: --long prints a note on standard error.")
    }
}
