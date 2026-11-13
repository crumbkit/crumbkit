//! Failure codes, and the spelling that turns a code into text.

use crate::ToolName;
use crate::segment::is_code_segment;

mod runtime;
#[cfg(test)]
mod tests;

pub use runtime::{CodeBuf, RuntimeCodeError, Segment};

/// The stable code of a failure, such as `shelf::books::not_found` in the
/// colon spelling.
///
/// A code has three segments: the component that owns the code, the area of
/// the tool it belongs to, and the reason. A tool declares each code as a
/// constant and returns it from [`Failure::code`](crate::Failure::code).
///
/// A code borrows its three segments for the lifetime `'a`. A declared
/// constant is a `Code<'static>`. A code that the tool learns at run time,
/// from [`Code::runtime`], borrows the text it was made from, and a value
/// that keeps such a code holds a [`CodeBuf`].
///
/// `Code` has no `Display`, so every printed code goes through [`Spelling`].
///
/// ```
/// use crumbkit::{Code, Spelling};
///
/// const NOT_FOUND: Code<'static> = Code::new("shelf", "books", "not_found");
///
/// assert_eq!(Spelling::COLONS.spell(NOT_FOUND), "shelf::books::not_found");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Code<'a> {
    /// The component that owns the code, such as `shelf`.
    component: &'a str,
    /// The area of the tool, such as `books`.
    area: &'a str,
    /// The reason for the failure, such as `not_found`.
    reason: &'a str,
}

impl<'a> Code<'a> {
    /// Returns the code with the segments `component`, `area` and `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless each segment is one or more words joined by single
    /// hyphens, where each word is lowercase ASCII letters, digits and
    /// underscores, such as `not_found` or `book-club`. A hyphen never leads,
    /// trails or follows another hyphen. A `const` that calls it with any
    /// other segment fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Code<'static> = crumbkit::Code::new("shelf", "books", "-not-found");
    /// ```
    ///
    /// A tool that learns the segments at run time calls [`Code::runtime`],
    /// which returns an error in place of the panic.
    #[must_use]
    pub const fn new(component: &'a str, area: &'a str, reason: &'a str) -> Self {
        assert!(
            is_code_segment(component),
            "a code component is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
        );
        assert!(
            is_code_segment(area),
            "a code area is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
        );
        assert!(
            is_code_segment(reason),
            "a code reason is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
        );
        Self {
            component,
            area,
            reason,
        }
    }

    /// Returns the component that owns the code, such as `shelf`.
    #[must_use]
    pub const fn component(&self) -> &'a str {
        self.component
    }

    /// Returns the area of the tool that the code belongs to, such as
    /// `books`.
    #[must_use]
    pub const fn area(&self) -> &'a str {
        self.area
    }

    /// Returns the reason for the failure, such as `not_found`.
    #[must_use]
    pub const fn reason(&self) -> &'a str {
        self.reason
    }

    /// Returns the code with `component` in place of its own, which
    /// [`ToolName`] checked is a code component.
    pub(crate) const fn with_component(self, component: &'a str) -> Self {
        Self { component, ..self }
    }
}

/// How crumbkit spells a failure code as text.
///
/// crumbkit prints every code in the one built-in spelling,
/// [`Spelling::COLONS`], which joins the three segments of a code with `::`.
/// [`Spelling::for_tool`] names the tool whose name a run writes in place of
/// `crumbkit` in crumbkit's own codes.
#[derive(Clone, Copy, Debug)]
pub struct Spelling {
    /// The tool whose name the spelling writes in place of `crumbkit` in
    /// crumbkit's own codes.
    tool: ToolName,
}

/// The text between the segments of a spelled code.
const SEPARATOR: &str = "::";

impl Spelling {
    /// Joins the segments with `::`, as in `shelf::books::not_found`.
    pub const COLONS: Self = Self {
        tool: ToolName::CRUMBKIT,
    };

    /// Returns the same spelling, writing `tool`'s name in place of
    /// `crumbkit` in each of crumbkit's own codes, such as
    /// `shelf::cli::usage` for `crumbkit::cli::usage`. A code whose component
    /// is not `crumbkit` keeps it.
    ///
    /// ```
    /// use crumbkit::{Code, Spelling, ToolName};
    ///
    /// const USAGE: Code<'static> = Code::new("crumbkit", "cli", "usage");
    ///
    /// let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));
    /// assert_eq!(spelling.spell(USAGE), "shelf::cli::usage");
    /// ```
    #[must_use]
    pub const fn for_tool(self, tool: ToolName) -> Self {
        Self { tool }
    }

    /// Returns the tool whose name the spelling writes in crumbkit's own
    /// codes, which is [`ToolName::CRUMBKIT`] until
    /// [`Spelling::for_tool`] names one.
    #[must_use]
    pub const fn tool(self) -> ToolName {
        self.tool
    }

    /// Returns `code` as text in this spelling.
    ///
    /// `crumbkit::finish` prints a failure's code this way, and a test
    /// calls it to check how a tool's codes print.
    #[must_use]
    pub fn spell(self, code: Code<'_>) -> String {
        let code = self.tool.code(code);
        format!(
            "{}{SEPARATOR}{}{SEPARATOR}{}",
            code.component, code.area, code.reason
        )
    }

    /// Returns the component, the area and the reason of `text`, a code in
    /// this spelling, or `None` when `text` does not split into exactly three
    /// parts at `::`.
    ///
    /// It does not check the segments, so a tool passes them to
    /// [`Code::runtime`], which does.
    ///
    /// ```
    /// use crumbkit::Spelling;
    ///
    /// assert_eq!(
    ///     Spelling::COLONS.split("lending::loans::overdue"),
    ///     Some(("lending", "loans", "overdue"))
    /// );
    /// assert_eq!(Spelling::COLONS.split("lending::overdue"), None);
    /// ```
    #[must_use]
    pub fn split(self, text: &str) -> Option<(&str, &str, &str)> {
        let mut parts = text.split(SEPARATOR);
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(component), Some(area), Some(reason), None) => Some((component, area, reason)),
            _ => None,
        }
    }
}
