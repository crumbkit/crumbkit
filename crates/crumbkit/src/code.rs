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
/// the tool it belongs to, and the reason. A code is its spelling, one text
/// with the segments joined by `::`, and the three segments are slices of
/// it. A tool declares each code as a constant and returns it from
/// [`Failure::code`](crate::Failure::code).
///
/// A code borrows its spelling for the lifetime `'a`. A declared constant is
/// a `Code<'static>`, and [`Code::as_str`] returns its spelling as a
/// `&'static str`, which a tool's library can take for its own trait. A code
/// that the tool learns at run time, from [`Code::runtime`], borrows the text
/// it was made from, and a value that keeps such a code holds a
/// [`CodeBuf`].
///
/// `Code` has no `Display`, so every printed code goes through [`Spelling`].
/// [`Code::as_str`] returns the declared text, so a code of crumbkit's own
/// reads `crumbkit::cli::usage` there, and the printed spelling writes the
/// tool's name in place of `crumbkit`.
///
/// ```
/// use crumbkit::{Code, Spelling};
///
/// const NOT_FOUND: Code<'static> = Code::from_spelling("shelf::books::not_found");
///
/// assert_eq!(NOT_FOUND.as_str(), "shelf::books::not_found");
/// assert_eq!(Spelling::COLONS.spell(NOT_FOUND), "shelf::books::not_found");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Code<'a> {
    /// The spelling, such as `shelf::books::not_found`.
    spelling: &'a str,
    /// The byte offset where the area starts, just after the first `::`.
    area: usize,
    /// The byte offset where the reason starts, just after the second `::`.
    reason: usize,
}

/// Why a text is no code spelling.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SpellingError {
    /// The text does not split into three parts at `::`.
    Parts,
    /// A segment breaks the rule of [`Code::from_spelling`].
    Segment(Segment),
}

/// Returns the offsets of the area and the reason in `spelling`, or why it
/// is no code spelling.
const fn split_spelling(spelling: &str) -> Result<(usize, usize), SpellingError> {
    let bytes = spelling.as_bytes();
    // A `const fn` cannot call iterator methods, so this loop walks the bytes
    // by index. A segment has no `:`, so the first `::` ends the component
    // and the second ends the area.
    let mut separators = [0usize; 2];
    let mut found = 0;
    let mut index = 0;
    while index + 1 < bytes.len() && found < 2 {
        if bytes[index] == b':' && bytes[index + 1] == b':' {
            separators[found] = index;
            found += 1;
            index += 2;
        } else {
            index += 1;
        }
    }
    if found < 2 {
        return Err(SpellingError::Parts);
    }
    let area = separators[0] + 2;
    let reason = separators[1] + 2;
    let (head, reason_text) = spelling.split_at(reason);
    let (component, area_text) = head.split_at(separators[0]);
    // `area_text` still starts with the first `::` and ends with the second.
    let (_, area_text) = area_text.split_at(2);
    let (area_text, _) = area_text.split_at(area_text.len() - 2);
    if !is_code_segment(component) {
        return Err(SpellingError::Segment(Segment::Component));
    }
    if !is_code_segment(area_text) {
        return Err(SpellingError::Segment(Segment::Area));
    }
    if !is_code_segment(reason_text) {
        // A fourth part leaves a `::` in the reason.
        return if contains_separator(reason_text) {
            Err(SpellingError::Parts)
        } else {
            Err(SpellingError::Segment(Segment::Reason))
        };
    }
    Ok((area, reason))
}

/// Returns whether `text` holds `::`.
const fn contains_separator(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b':' && bytes[index + 1] == b':' {
            return true;
        }
        index += 1;
    }
    false
}

impl<'a> Code<'a> {
    /// Returns the code with the spelling `spelling`, such as
    /// `shelf::books::not_found`.
    ///
    /// # Panics
    ///
    /// Panics unless `spelling` is three segments joined by `::`, where each
    /// segment is one or more words joined by single hyphens, and each word
    /// is lowercase ASCII letters, digits and underscores, such as
    /// `not_found` or `book-club`. A hyphen never leads, trails or follows
    /// another hyphen. A `const` that calls it with any other text fails to
    /// compile, and the message names the broken part:
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Code<'static> = crumbkit::Code::from_spelling("shelf::books::-not-found");
    /// ```
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Code<'static> = crumbkit::Code::from_spelling("shelf::not_found");
    /// ```
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Code<'static> = crumbkit::Code::from_spelling("shelf::books::not::found");
    /// ```
    ///
    /// ```compile_fail
    /// const BAD: crumbkit::Code<'static> = crumbkit::Code::from_spelling("shelf::::not_found");
    /// ```
    ///
    /// Declare each code in a `const`, so that a bad spelling stops the
    /// build and never a run. A tool that learns the spelling at run time
    /// calls [`Code::runtime`], which returns an error in place of the
    /// panic.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a const fn reports a bad spelling by panicking, which fails the build of a const"
    )]
    pub const fn from_spelling(spelling: &'a str) -> Self {
        match split_spelling(spelling) {
            Ok((area, reason)) => Self {
                spelling,
                area,
                reason,
            },
            Err(SpellingError::Parts) => panic!(
                "a code spelling is three segments joined by two `::`, such as `shelf::books::not_found`"
            ),
            Err(SpellingError::Segment(Segment::Component)) => panic!(
                "a code component is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
            Err(SpellingError::Segment(Segment::Area)) => panic!(
                "a code area is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
            Err(SpellingError::Segment(Segment::Reason)) => panic!(
                "a code reason is words of lowercase ASCII letters, digits and underscores joined by single hyphens"
            ),
        }
    }

    /// Returns the spelling of the code as it was declared, such as
    /// `shelf::books::not_found`.
    ///
    /// A code of crumbkit's own returns `crumbkit::cli::usage`. The printed
    /// spelling, [`Spelling::spell`], writes the tool's name in place of
    /// `crumbkit`.
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        self.spelling
    }

    /// Returns the component that owns the code, such as `shelf`.
    #[must_use]
    pub const fn component(&self) -> &'a str {
        self.spelling.split_at(self.area - SEPARATOR.len()).0
    }

    /// Returns the area of the tool that the code belongs to, such as
    /// `books`.
    #[must_use]
    pub const fn area(&self) -> &'a str {
        let (head, _) = self.spelling.split_at(self.reason - SEPARATOR.len());
        head.split_at(self.area).1
    }

    /// Returns the reason for the failure, such as `not_found`.
    #[must_use]
    pub const fn reason(&self) -> &'a str {
        self.spelling.split_at(self.reason).1
    }

    /// Returns the spelling after the component, such as
    /// `::books::not_found`.
    const fn after_component(&self) -> &'a str {
        self.spelling.split_at(self.area - SEPARATOR.len()).1
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
    /// const USAGE: Code<'static> = Code::from_spelling("crumbkit::cli::usage");
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
        format!(
            "{}{}",
            self.tool.code_component(code),
            code.after_component()
        )
    }
}
