//! The names that crumbkit gives its own codes, documents and variables in
//! one tool's output.

use std::sync::{Mutex, PoisonError};

use crate::segment::is_component;
use crate::{Code, Document};

#[cfg(test)]
mod tests;

/// The component of crumbkit's own codes and documents, such as
/// `crumbkit::cli::usage` and `crumbkit/failure/v1`, which a tool's output
/// spells with the tool's name.
const CRUMBKIT: &str = "crumbkit";

/// The name that a tool's output gives crumbkit's own failure codes,
/// document ids and environment variables, derived from the tool's name.
///
/// A tool's output names the tool and never crumbkit. crumbkit derives the
/// name from the tool's root command, as `crumbkit::help::Guide::tool_name`
/// describes, and spells it as each place needs:
///
/// - a document id keeps the name as it is, such as `my-tool/failure/v1`;
/// - a failure code's component has `_` in place of each `-` and of each
///   other character that a code segment does not allow, such as
///   `my_tool::cli::usage`;
/// - an environment variable starts with the code's component in capitals,
///   such as `MY_TOOL_OUTPUT_BUDGET`, and with `_` before a leading digit,
///   such as `_2FA_OUTPUT_BUDGET`, because a shell variable cannot start
///   with a digit.
///
/// An id and a code lowercase capitals. A document id has `-` in place of
/// each character other than a letter, a digit, `_`, `-` and `.`, and in
/// place of its dots too when they still break the rule of a document
/// component. A name without an ASCII letter or digit is spelled `tool` in
/// every place.
///
/// Only crumbkit's own codes and documents change, which are those whose
/// component is `crumbkit`. A tool's own codes and documents keep the
/// component that the tool gave them.
///
/// ```
/// use crumbkit::{Code, Spelling, ToolName};
///
/// const USAGE: Code = Code::new("crumbkit", "cli", "usage");
///
/// let tool = ToolName::new("my-tool");
/// assert_eq!(Spelling::COLONS.spell(&tool.code(&USAGE)), "my_tool::cli::usage");
/// assert_eq!(tool.schema_id("crumbkit/failure/v1"), "my-tool/failure/v1");
/// assert_eq!(tool.variable("OUTPUT_BUDGET"), "MY_TOOL_OUTPUT_BUDGET");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToolName {
    /// The name as the tool gives it, such as `my-tool`.
    name: &'static str,
    /// The component of a document id, such as `my-tool`.
    schema: &'static str,
    /// The component of a failure code, such as `my_tool`.
    code: &'static str,
    /// The start of an environment variable's name, such as `MY_TOOL`.
    variable: &'static str,
}

/// The tool names that this process has derived, so each distinct name is
/// spelled and kept once.
static NAMES: Mutex<Vec<ToolName>> = Mutex::new(Vec::new());

impl ToolName {
    /// The names that crumbkit's codes and documents have before a tool
    /// names them: `crumbkit` in every place. A [`Spelling`](crate::Spelling)
    /// that no tool named spells crumbkit's codes with it.
    pub const CRUMBKIT: Self = Self {
        name: CRUMBKIT,
        schema: CRUMBKIT,
        code: CRUMBKIT,
        variable: "CRUMBKIT",
    };

    /// Returns the names of the tool whose name is `name`, spelled as the
    /// type's documentation describes.
    ///
    /// The process keeps each distinct name that it is given until it
    /// exits, because the segments of a [`Code`] live that long. A tool's
    /// names come from its own command tree, so a process holds one or a
    /// few of them.
    #[must_use]
    pub fn new(name: &str) -> Self {
        // The list holds only finished names, so it is whole even after a
        // thread panicked while it held the lock, and a poisoned lock is
        // used as it is.
        let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(held) = names.iter().find(|held| held.name == name) {
            return *held;
        }
        let spelled = Self::spelled(name);
        names.push(spelled);
        spelled
    }

    /// Returns the names of `name`, with each spelling kept until the
    /// process exits.
    fn spelled(name: &str) -> Self {
        if !name.bytes().any(|byte| byte.is_ascii_alphanumeric()) {
            return Self {
                name: leak(name.to_owned()),
                schema: "tool",
                code: "tool",
                variable: "TOOL",
            };
        }
        let code = replaced(name, |byte| byte == b'_', b'_');
        let mut schema = replaced(name, |byte| matches!(byte, b'_' | b'-' | b'.'), b'-');
        if !is_component(&schema) {
            schema = schema.replace('.', "-");
        }
        // A shell variable cannot start with a digit, so such a name's
        // variables start with `_`, such as `_2FA_OUTPUT_BUDGET`.
        let mut variable = code.to_ascii_uppercase();
        if variable.starts_with(|first: char| first.is_ascii_digit()) {
            variable.insert(0, '_');
        }
        Self {
            name: leak(name.to_owned()),
            schema: leak(schema),
            variable: leak(variable),
            code: leak(code),
        }
    }

    /// Returns the name as the tool gives it, such as `my-tool`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns `code` with the tool's component in place of `crumbkit`,
    /// or `code` itself when the tool or another component owns it.
    #[must_use]
    pub const fn code(self, code: &Code) -> Code {
        if is_crumbkit(code.component()) {
            code.with_component(self.code)
        } else {
            *code
        }
    }

    /// Returns `document` with the tool's component in place of
    /// `crumbkit`, or `document` itself when another component owns it.
    #[must_use]
    pub const fn document<T>(self, document: Document<T>) -> Document<T> {
        if is_crumbkit(document.component()) {
            document.with_component(self.schema)
        } else {
            document
        }
    }

    /// Returns the document id `id`, such as `crumbkit/failure/v1`, with
    /// the tool's component in place of `crumbkit`, or `id` unchanged when
    /// it does not start with `crumbkit/`.
    #[must_use]
    pub fn schema_id(self, id: &str) -> String {
        match id.strip_prefix("crumbkit/") {
            Some(rest) => format!("{}/{rest}", self.schema),
            None => id.to_owned(),
        }
    }

    /// Returns the name of the tool's environment variable that ends in
    /// `suffix`, such as `SHELF_OUTPUT_BUDGET` for `OUTPUT_BUDGET`.
    #[must_use]
    pub fn variable(self, suffix: &str) -> String {
        format!("{}_{suffix}", self.variable)
    }
}

/// Reports whether `component` is `crumbkit`, the component of crumbkit's
/// own codes and documents.
const fn is_crumbkit(component: &str) -> bool {
    let (left, right) = (component.as_bytes(), CRUMBKIT.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    // A `const fn` cannot compare slices, so this loop walks the bytes by
    // index.
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns `name` lowercased, with each byte that is not an ASCII letter, a
/// digit or a byte that `keeps` accepts replaced by `with`. Each byte of a
/// character outside ASCII is replaced, so such a character becomes one
/// `with` for each of its bytes.
fn replaced(name: &str, keeps: impl Fn(u8) -> bool, with: u8) -> String {
    name.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .map(|byte| {
            if byte.is_ascii_lowercase() || byte.is_ascii_digit() || keeps(byte) {
                char::from(byte)
            } else {
                char::from(with)
            }
        })
        .collect()
}

/// Returns `text` as a string that lives until the process exits.
fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}
