//! [`Line`], the text line facet of a row type, which
//! [`Row::LINE`](crate::Row::LINE) holds (ADR-0064, ADR-0213).

use std::fmt;

/// The form of each row's own text line as help shows it, and the function
/// that builds the line of one row of the row type `R` (ADR-0064,
/// ADR-0213).
///
/// When a row type sets [`Row::LINE`](crate::Row::LINE), the default text
/// prints each row's line in place of the table, without a header.
/// `--long` and `--fields` still print the table, because they name
/// columns, and `--json` is unchanged. The leaf's help prints the form on a
/// `Text:` line. crumbkit-test's `check_help` fails a form that is not one
/// non-empty line, and a headed leaf whose row type sets it, because a
/// headed batch's text prints each row's bytes.
///
/// The function returns the line without a line feed, or `None` when the
/// row has none. crumbkit escapes each control character of the line, such
/// as a line feed as `\n`, and each Unicode format character, such as a
/// bidirectional override as `\u{202e}`. It ends the line with a line feed,
/// so each row stays one line and shows its text in the order it was
/// written. A row whose function returns `None` prints as one line of
/// compact JSON with every field.
///
/// ```
/// use crumbkit::{Line, Row};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Link {
///     pointer: String,
///     target: Option<String>,
/// }
///
/// impl Link {
///     // A link without a target has no line, so it prints as JSON.
///     fn line(&self) -> Option<String> {
///         let target = self.target.as_ref()?;
///         Some(format!("{} -> {target}", self.pointer))
///     }
/// }
///
/// impl Row for Link {
///     const COLUMNS: &'static [&'static str] = &["pointer", "target"];
///     const LINE: Option<Line<Self>> = Some(Line::new("<POINTER> -> <TARGET>", Self::line));
/// }
///
/// let link = Link { pointer: "latest".to_owned(), target: Some("v2".to_owned()) };
/// let line = Link::LINE.and_then(|line| line.text(&link));
/// assert_eq!(line.as_deref(), Some("latest -> v2"));
/// ```
// `Clone`, `Copy` and `Debug` are written by hand, because their derives
// would bound `R`, which a function pointer does not need. A function pointer
// has no reliable equality, so `Line` has no `PartialEq`.
pub struct Line<R: ?Sized> {
    /// The form of the line as help shows it.
    form: &'static str,
    /// Builds one row's line.
    text: fn(&R) -> Option<String>,
}

impl<R: ?Sized> Line<R> {
    /// Returns the line whose form help shows as `form`, such as
    /// `<POINTER> -> <TARGET>`, and whose text `text` builds for each row.
    #[must_use]
    pub const fn new(form: &'static str, text: fn(&R) -> Option<String>) -> Self {
        Self { form, text }
    }

    /// Returns the form of the line as help shows it.
    #[must_use]
    pub const fn get_form(self) -> &'static str {
        self.form
    }

    /// Returns the line of `row`, without a line feed, or `None` when the
    /// row has none.
    #[must_use]
    pub fn text(self, row: &R) -> Option<String> {
        (self.text)(row)
    }
}

impl<R: ?Sized> Clone for Line<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for Line<R> {}

impl<R: ?Sized> fmt::Debug for Line<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Line")
            .field("form", &self.form)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
