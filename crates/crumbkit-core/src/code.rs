//! Failure codes, and the spellings that turn a code into text.

use std::fmt;

use crate::ToolName;
use crate::segment::is_segment;

mod runtime;
#[cfg(test)]
mod tests;

pub use runtime::RuntimeCodeError;

/// The stable code of a failure, such as `shelf::books::not_found` in the
/// colon spelling.
///
/// A code has three segments: the component that owns the code, the area of
/// the tool it belongs to, and the reason. A tool declares each code as a
/// constant and returns it from [`Failure::code`](crate::Failure::code).
///
/// `Code` has no `Display`. Each tool chooses how its codes print with one
/// [`Spelling`], so every printed code goes through that spelling.
///
/// ```
/// use crumbkit_core::{Code, Spelling};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
///
/// assert_eq!(Spelling::COLONS.spell(&NOT_FOUND), "shelf::books::not_found");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Code {
    /// The component that owns the code, such as `shelf`.
    component: &'static str,
    /// The area of the tool, such as `books`.
    area: &'static str,
    /// The reason for the failure, such as `not_found`.
    reason: &'static str,
}

impl Code {
    /// Returns the code with the segments `component`, `area` and `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless each segment is non-empty lowercase ASCII letters,
    /// digits and underscores. A `const` that calls it with such a segment
    /// fails to compile:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit_core::Code = crumbkit_core::Code::new("shelf", "books", "not-found");
    /// ```
    #[must_use]
    pub const fn new(component: &'static str, area: &'static str, reason: &'static str) -> Self {
        assert!(
            is_segment(component, b'_'),
            "a code component is non-empty lowercase ASCII letters, digits and underscores"
        );
        assert!(
            is_segment(area, b'_'),
            "a code area is non-empty lowercase ASCII letters, digits and underscores"
        );
        assert!(
            is_segment(reason, b'_'),
            "a code reason is non-empty lowercase ASCII letters, digits and underscores"
        );
        Self {
            component,
            area,
            reason,
        }
    }

    /// Returns the component that owns the code, such as `shelf`.
    #[must_use]
    pub const fn component(&self) -> &'static str {
        self.component
    }

    /// Returns the area of the tool that the code belongs to, such as
    /// `books`.
    #[must_use]
    pub const fn area(&self) -> &'static str {
        self.area
    }

    /// Returns the reason for the failure, such as `not_found`.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Returns the code with `component` in place of its own, which
    /// [`ToolName`] checked is a code component.
    pub(crate) const fn with_component(self, component: &'static str) -> Self {
        Self { component, ..self }
    }
}

/// How a tool spells its failure codes as text.
///
/// A tool sets one spelling on its guide with `Guide::spelling`, and
/// crumbkit prints every code in it. crumbkit-test's codes check reads the
/// same spelling from the guide. The built-in spelling,
/// [`Spelling::COLONS`], joins the three segments of a code with `::`, and
/// a guide that sets no spelling uses it. A tool that needs another form,
/// such as segments joined with `.`, gives its own function to
/// [`Spelling::custom`].
#[derive(Clone, Copy, Debug)]
pub struct Spelling {
    /// Writes a code's text to the formatter.
    write: fn(&Code, &mut fmt::Formatter<'_>) -> fmt::Result,
    /// The text between the segments of the built-in spelling, which
    /// [`Spelling::split`] splits at, or `None` for a custom spelling.
    separator: Option<&'static str>,
    /// The tool whose name the spelling writes in place of `crumbkit` in
    /// crumbkit's own codes.
    tool: ToolName,
}

impl Spelling {
    /// Joins the segments with `::`, as in `shelf::books::not_found`.
    pub const COLONS: Self = Self::built_in("::", |code, f| {
        write!(f, "{}::{}::{}", code.component, code.area, code.reason)
    });

    /// Returns a spelling that writes each code with `write`, which reads the
    /// segments through [`Code::component`], [`Code::area`] and
    /// [`Code::reason`].
    ///
    /// crumbkit-test's codes check fails when two of a tool's codes spell the
    /// same, or when a spelled code is empty or holds a character other than
    /// printable ASCII, `!` to `~`.
    ///
    /// ```
    /// use std::fmt;
    ///
    /// use crumbkit_core::{Code, Spelling};
    ///
    /// fn upper(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///     write!(f, "{}_{}", code.component().to_uppercase(), code.reason().to_uppercase())
    /// }
    ///
    /// const UPPER: Spelling = Spelling::custom(upper);
    ///
    /// let code = Code::new("shelf", "books", "not_found");
    /// assert_eq!(UPPER.spell(&code), "SHELF_NOT_FOUND");
    /// ```
    #[must_use]
    pub const fn custom(write: fn(&Code, &mut fmt::Formatter<'_>) -> fmt::Result) -> Self {
        Self {
            write,
            separator: None,
            tool: ToolName::CRUMBKIT,
        }
    }

    /// Returns a built-in spelling that writes each code with `write`, which
    /// joins the segments with `separator`.
    const fn built_in(
        separator: &'static str,
        write: fn(&Code, &mut fmt::Formatter<'_>) -> fmt::Result,
    ) -> Self {
        Self {
            write,
            separator: Some(separator),
            tool: ToolName::CRUMBKIT,
        }
    }

    /// Returns the same spelling, writing `tool`'s name in place of
    /// `crumbkit` in each of crumbkit's own codes, such as
    /// `shelf::cli::usage` for `crumbkit::cli::usage`.
    ///
    /// crumbkit names the spelling for the tool it runs, so a tool sets its
    /// spelling on its guide without a tool name. A code whose component is
    /// not `crumbkit` keeps it.
    ///
    /// ```
    /// use crumbkit_core::{Code, Spelling, ToolName};
    ///
    /// const USAGE: Code = Code::new("crumbkit", "cli", "usage");
    ///
    /// let spelling = Spelling::COLONS.for_tool(ToolName::new("shelf"));
    /// assert_eq!(spelling.spell(&USAGE), "shelf::cli::usage");
    /// ```
    #[must_use]
    pub const fn for_tool(self, tool: ToolName) -> Self {
        Self { tool, ..self }
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
    pub fn spell(self, code: &Code) -> String {
        Spelled {
            spelling: self,
            code,
        }
        .to_string()
    }

    /// Returns the component, the area and the reason of `text`, a code in
    /// this spelling, or `None` when this is a custom spelling or `text`
    /// does not split into exactly three parts at the spelling's separator.
    ///
    /// It does not check the segments, so a tool passes them to
    /// [`Code::runtime`], which does. A custom spelling may drop a segment or
    /// change its letters, so it cannot be read back. A tool with a custom
    /// spelling therefore splits the text itself before it calls
    /// [`Code::runtime`], because this method returns `None` for every code.
    ///
    /// ```
    /// use crumbkit_core::Spelling;
    ///
    /// assert_eq!(
    ///     Spelling::COLONS.split("lending::loans::overdue"),
    ///     Some(("lending", "loans", "overdue"))
    /// );
    /// assert_eq!(Spelling::COLONS.split("lending::overdue"), None);
    /// ```
    #[must_use]
    pub fn split(self, text: &str) -> Option<(&str, &str, &str)> {
        let mut parts = text.split(self.separator?);
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(component), Some(area), Some(reason), None) => Some((component, area, reason)),
            _ => None,
        }
    }
}

/// A code in a spelling, which `Display` writes. A spelling's function needs a
/// `fmt::Formatter`, and only a `Display` impl receives one.
struct Spelled<'a> {
    /// The spelling to write the code in.
    spelling: Spelling,
    /// The code to write.
    code: &'a Code,
}

impl fmt::Display for Spelled<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.spelling.write)(&self.spelling.tool.code(self.code), f)
    }
}
