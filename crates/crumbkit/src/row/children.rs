//! The default child lines of a row: the text of the items of its children
//! field.

use serde::Serialize;
use serde_json::Value;

use crate::shown::TextCells;

/// A row type's child lines, which [`Row::CHILDREN`](crate::Row::CHILDREN)
/// holds (ADR-0120, ADR-0182).
///
/// ```
/// use crumbkit::Children;
///
/// const NOTES: Children = Children::field("notes")
///     .indent(0)
///     .spaced()
///     .help("one line for each note.");
///
/// assert_eq!(NOTES.get_field(), "notes");
/// assert_eq!(NOTES.get_indent(), 0);
/// assert!(NOTES.is_spaced());
/// assert_eq!(NOTES.get_help(), Some("one line for each note."));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Children {
    /// The field that holds the children, as the type serializes it.
    field: &'static str,
    /// The number of spaces before each child line in text.
    indent: u8,
    /// Whether a blank line follows a row that printed child lines.
    spaced: bool,
    /// The description of the child lines in help, or `None` for
    /// crumbkit's sentence.
    help: Option<&'static str>,
}

impl Children {
    /// Returns the child lines of the field `field`, as the type serializes
    /// it, after any `#[serde(rename)]`. They are indented by 2 spaces, have
    /// no blank line after them and take crumbkit's sentence in help.
    #[must_use]
    pub const fn field(field: &'static str) -> Self {
        Self {
            field,
            indent: 2,
            spaced: false,
            help: None,
        }
    }

    /// Sets the number of spaces before each of the row's
    /// [`Row::child_lines`](crate::Row::child_lines) in text (ADR-0135).
    ///
    /// A type that sets 0 prints each child line at column 1, as a table
    /// line starts, so a reader tells the two apart only by the line's
    /// content, which the doc comment of the children field should describe.
    /// The type is `u8`, so the compiler rejects an indent above 255. The
    /// output budget counts the spaces. Help's own `Children:` sentence
    /// calls the lines indented only when the indent is above 0 (ADR-0137).
    #[must_use]
    pub const fn indent(mut self, spaces: u8) -> Self {
        self.indent = spaces;
        self
    }

    /// Prints a blank line after a row that printed child lines, when
    /// another row follows, so that each row reads as one block with its
    /// children (ADR-0151).
    ///
    /// The blank line prints wherever the text prints child lines: under a
    /// table row, with `--long`, and under a row's own text line when the
    /// type sets [`Row::LINE`](crate::Row::LINE). A row without child lines,
    /// and the last row that prints, are followed by no blank line.
    /// `--fields`, `--json` and a stream do not change. The output budget
    /// counts each blank line.
    #[must_use]
    pub const fn spaced(mut self) -> Self {
        self.spaced = true;
        self
    }

    /// Sets the description of the child lines that help prints after
    /// `Children: `, ending with a full stop (ADR-0137).
    ///
    /// crumbkit's sentence is `one indented line under each row for each of
    /// its <FIELD>.`, without `indented` when [`Children::indent`] set 0. A
    /// type whose [`Row::child_lines`](crate::Row::child_lines) are not one
    /// line per item sets its own text, such as `one line for each
    /// application, or one folded line when every application is stopped.`.
    /// `-h`, `--help`, the `children_help` field of `commands list` and
    /// crumbkit-markdown's reference print the same text. crumbkit-test's
    /// `check_help` fails a text that is empty, holds a control character or
    /// does not end with a full stop.
    #[must_use]
    pub const fn help(mut self, text: &'static str) -> Self {
        self.help = Some(text);
        self
    }

    /// Returns the field that holds the children.
    #[must_use]
    pub const fn get_field(self) -> &'static str {
        self.field
    }

    /// Returns the number of spaces before each child line.
    #[must_use]
    pub const fn get_indent(self) -> u8 {
        self.indent
    }

    /// Reports whether a blank line follows a row that printed child lines.
    #[must_use]
    pub const fn is_spaced(self) -> bool {
        self.spaced
    }

    /// Returns the description that [`Children::help`] set, or `None` for
    /// crumbkit's sentence.
    #[must_use]
    pub const fn get_help(self) -> Option<&'static str> {
        self.help
    }

    /// Returns the text that help prints after `Children: `.
    pub(crate) fn text(self) -> String {
        self.help.map_or_else(|| self.sentence(), str::to_owned)
    }

    /// Returns crumbkit's description of the child lines. The sentence calls
    /// the lines indented only when the indent is above 0, because a line at
    /// column 1 is not indented.
    fn sentence(self) -> String {
        let indented = if self.indent == 0 { "" } else { "indented " };
        format!(
            "one {indented}line under each row for each of its {}.",
            self.field
        )
    }
}

#[cfg(test)]
mod tests;

/// Returns the items of the field `children` of `row`, serialized as text
/// cells are, when the field holds an array whose every item is a string,
/// and no lines otherwise. A row without a children field, or one that does
/// not serialize, has no lines.
//
// RUST-10: this serializes the whole row a second time, into a
// `serde_json::Value`, for each row whose type sets `Row::CHILDREN` and keeps
// the default `child_lines`, although the table, `Room` and a stream's text
// line have already serialized that row in the text mode. A row type without
// `CHILDREN` never reaches it, and a type that overrides `child_lines` pays
// nothing. Reusing the value that the caller already holds needs the caller
// to know whether a type overrides `child_lines`, which Rust cannot tell
// without specialization or a new public method on `Row`. A `Row` method
// that receives that value, whose default reads the children field from it
// and which `child_lines` overrides replace, removes the second
// serialization when a profile shows that it matters.
pub(super) fn shown_lines<R: Serialize + ?Sized>(row: &R, children: Option<&str>) -> Vec<String> {
    let Some(children) = children else {
        return Vec::new();
    };
    let value = {
        let _cells = TextCells::enter();
        serde_json::to_value(row)
    };
    let Ok(Value::Object(mut fields)) = value else {
        return Vec::new();
    };
    let Some(Value::Array(items)) = fields.remove(children) else {
        return Vec::new();
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(text) => Some(text),
            _ => None,
        })
        .collect::<Option<Vec<String>>>()
        .unwrap_or_default()
}
