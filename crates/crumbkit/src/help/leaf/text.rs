//! The `Text:` line of a leaf whose rows print their own text line
//! (ADR-0064).

use super::Leaf;
use crate::help::ViewFlag;

#[cfg(test)]
mod tests;

/// Returns the `Text:` line that help prints for rows whose own text line
/// has the form `form`, such as `Text: one line per row, <POINTER> ->
/// <TARGET>; --fields and --long print a table.`. `fields` and `long` say
/// whether the command offers `--fields` and `--long`, and the line names
/// only the flags that it offers. A command that offers neither, such as
/// one whose rows print whole and name no columns, gets a line that names
/// no table. crumbkit-markdown prints the same line in a reference.
#[must_use]
pub fn text_line(form: &str, fields: bool, long: bool) -> String {
    let flags = match (fields, long) {
        (true, true) => "; --fields and --long print a table",
        (true, false) => "; --fields prints a table",
        (false, true) => "; --long prints a table",
        (false, false) => "",
    };
    format!("Text: one line per row, {form}{flags}.")
}

impl Leaf {
    /// Returns the form of the text line that each row of the leaf's
    /// document prints, the row type's [`Row::LINE`](crate::Row::LINE), or
    /// `None` when the leaf has no document or its rows print a table.
    #[must_use]
    pub const fn line(&self) -> Option<&'static str> {
        match self.columns {
            Some(columns) => columns.line,
            None => None,
        }
    }

    /// Returns the `Text:` line that the leaf's `-h` and `--help` print
    /// after the `Output:` line, as [`text_line`] builds it, or `None` when
    /// the leaf's rows print a table. A headed leaf has none, because its
    /// text prints each row's bytes.
    pub(crate) fn text_help_line(&self) -> Option<String> {
        let form = self.line()?;
        if self.get_document()?.is_headed() {
            return None;
        }
        Some(text_line(
            form,
            self.offers(ViewFlag::Fields),
            self.offers(ViewFlag::Long),
        ))
    }
}
