//! [`Children`], the child lines facet of a row type, which
//! [`Row::CHILDREN`](crate::Row::CHILDREN) holds (ADR-0120, ADR-0214).

use std::fmt;

use serde::Serialize;
use serde_json::Value;

use crate::shown::text_cells;

/// The child lines of a row of the row type `R`: the field that holds the
/// children, how the text prints them, their help and the functions that
/// build them (ADR-0120, ADR-0182, ADR-0214).
///
/// When a row type sets [`Row::CHILDREN`](crate::Row::CHILDREN), the text
/// prints each row's lines under the row's table line, or under its own
/// text line when the type sets [`Row::LINE`](crate::Row::LINE), each after
/// [`Children::indent`] spaces and followed by a line feed. crumbkit
/// escapes each control character and each Unicode format character of a
/// line as a table cell escapes them, so each child stays on one line. A
/// row with no lines prints nothing under it. The child lines do not change
/// the widths of the table's columns, and the output budget counts them.
///
/// Without [`Children::lines`], the lines are the items of the field when
/// it is an array of strings, serialized as the row's text cells are, and
/// no lines otherwise. A field whose items are [`Shown`](crate::Shown)
/// values, or strings, therefore prints one child line for each item, its
/// `Display`:
///
/// ```
/// use std::fmt;
///
/// use crumbkit::{Children, Row, Shown};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// /// One part of a service, which prints as `+ database` when it answers.
/// struct Part {
///     name: &'static str,
///     up: bool,
/// }
///
/// impl fmt::Display for Part {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         let mark = if self.up { '+' } else { '-' };
///         write!(f, "{mark} {}", self.name)
///     }
/// }
///
/// impl Shown for Part {}
/// crumbkit::shown!(Part);
///
/// /// A service and the parts it runs on.
/// #[derive(Serialize, JsonSchema)]
/// struct Service {
///     /// The service's name.
///     name: String,
///     /// The parts that the service runs on.
///     parts: Vec<Part>,
/// }
///
/// impl Row for Service {
///     const COLUMNS: &'static [&'static str] = &["name"];
///     const CHILDREN: Option<Children<Self>> = Some(Children::field("parts").indent(4));
/// }
///
/// let service = Service {
///     name: "catalogue".to_owned(),
///     parts: vec![Part { name: "database", up: true }],
/// };
/// let lines = Service::CHILDREN.map(|children| children.lines_for(&service));
/// assert_eq!(lines.unwrap_or_default(), ["+ database"]);
/// ```
// `Clone`, `Copy` and `Debug` are written by hand, because their derives
// would bound `R`, which a function pointer does not need. A function pointer
// has no reliable equality, so `Children` has no `PartialEq`.
pub struct Children<R: ?Sized> {
    /// The field that holds the children, as the type serializes it.
    field: &'static str,
    /// The number of spaces before each child line in text.
    indent: u8,
    /// Whether a blank line follows a row that printed child lines.
    spaced: bool,
    /// The description of the child lines in help, or `None` for
    /// crumbkit's sentence.
    help: Option<&'static str>,
    /// Builds one row's lines, or `None` for the items of the field.
    lines: Option<fn(&R) -> Vec<String>>,
    /// Builds one row's lines for `--long`, or `None` for the compact
    /// lines.
    long_lines: Option<fn(&R) -> Vec<String>>,
}

impl<R: ?Sized> Children<R> {
    /// Returns the child lines of the field `field`, as the type serializes
    /// it, after any `#[serde(rename)]`. They are the field's items,
    /// indented by 2 spaces, have no blank line after them and take
    /// crumbkit's sentence in help.
    #[must_use]
    pub const fn field(field: &'static str) -> Self {
        Self {
            field,
            indent: 2,
            spaced: false,
            help: None,
            lines: None,
            long_lines: None,
        }
    }

    /// Sets the function that builds each row's child lines, one for each
    /// child, without indentation or a line feed, in place of the items of
    /// the field.
    ///
    /// ```
    /// use crumbkit::{Children, Row};
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// /// One part of a service.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Component {
    ///     /// The part's name.
    ///     name: String,
    ///     /// Whether the part answers.
    ///     up: bool,
    /// }
    ///
    /// /// A service and the parts it runs on.
    /// #[derive(Serialize, JsonSchema)]
    /// struct Service {
    ///     /// The service's name.
    ///     name: String,
    ///     /// The parts that the service runs on.
    ///     components: Vec<Component>,
    /// }
    ///
    /// impl Service {
    ///     fn components(&self) -> Vec<String> {
    ///         let mark = |up: bool| if up { '+' } else { '-' };
    ///         self.components
    ///             .iter()
    ///             .map(|part| format!("{} {}", mark(part.up), part.name))
    ///             .collect()
    ///     }
    /// }
    ///
    /// impl Row for Service {
    ///     const COLUMNS: &'static [&'static str] = &["name"];
    ///     const CHILDREN: Option<Children<Self>> =
    ///         Some(Children::field("components").lines(Self::components));
    /// }
    /// ```
    #[must_use]
    pub const fn lines(mut self, lines: fn(&R) -> Vec<String>) -> Self {
        self.lines = Some(lines);
        self
    }

    /// Sets the function that builds each row's child lines for `--long`,
    /// in place of the compact lines, which `--long` prints otherwise
    /// (ADR-0138).
    #[must_use]
    pub const fn long_lines(mut self, lines: fn(&R) -> Vec<String>) -> Self {
        self.long_lines = Some(lines);
        self
    }

    /// Sets the number of spaces before each of the row's child lines in
    /// text (ADR-0135).
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
    /// type whose [`Children::lines`] are not one line per item sets its own
    /// text, such as `one line for each application, or one folded line when
    /// every application is stopped.`. `-h`, `--help`, the `children_help`
    /// field of `commands list` and crumbkit-markdown's reference print the
    /// same text. crumbkit-test's `check_help` fails a text that is empty,
    /// holds a control character or does not end with a full stop.
    #[must_use]
    pub const fn help(mut self, text: &'static str) -> Self {
        self.help = Some(text);
        self
    }

    /// Returns the settings without the functions, as
    /// [`LeafDocument::children`](crate::help::LeafDocument::children)
    /// holds them for help.
    #[must_use]
    pub const fn untyped(self) -> Children<()> {
        Children {
            field: self.field,
            indent: self.indent,
            spaced: self.spaced,
            help: self.help,
            lines: None,
            long_lines: None,
        }
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

    /// Returns the text that help prints after `Children: `: the text that
    /// [`Children::help`] set, and otherwise crumbkit's sentence, such as
    /// `one indented line under each row for each of its components.`, which
    /// leaves out `indented` when [`Children::indent`] set 0. `commands list`
    /// writes it as `children_help`.
    #[must_use]
    pub fn text(self) -> String {
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

impl<R: Serialize + ?Sized> Children<R> {
    /// Returns the child lines of `row`: what [`Children::lines`] builds,
    /// and otherwise the items of the field.
    #[must_use]
    pub fn lines_for(self, row: &R) -> Vec<String> {
        match self.lines {
            Some(lines) => lines(row),
            None => shown_lines(row, self.field),
        }
    }

    /// Returns the child lines of `row` for `--long`: what
    /// [`Children::long_lines`] builds, and otherwise
    /// [`Children::lines_for`].
    #[must_use]
    pub fn long_lines_for(self, row: &R) -> Vec<String> {
        match self.long_lines {
            Some(lines) => lines(row),
            None => self.lines_for(row),
        }
    }
}

impl<R: ?Sized> Clone for Children<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R: ?Sized> Copy for Children<R> {}

impl<R: ?Sized> fmt::Debug for Children<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Children")
            .field("field", &self.field)
            .field("indent", &self.indent)
            .field("spaced", &self.spaced)
            .field("help", &self.help)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;

/// Returns the items of the field `children` of `row`, serialized as text
/// cells are, when the field holds an array whose every item is a string,
/// and no lines otherwise. A row without the field, or one that does not
/// serialize, has no lines.
//
// RUST-10: this serializes the whole row a second time, into a
// `serde_json::Value`, for each row whose type sets `Row::CHILDREN` without
// `Children::lines`, although the table, `Room` and a stream's text line
// have already serialized that row in the text mode. A row type without
// `CHILDREN` never reaches it, and a type that sets `lines` pays nothing.
// Passing the value that the caller already holds to `Children::lines_for`
// removes the second serialization when a profile shows that it matters.
fn shown_lines<R: Serialize + ?Sized>(row: &R, children: &str) -> Vec<String> {
    let value = text_cells(|| serde_json::to_value(row));
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
