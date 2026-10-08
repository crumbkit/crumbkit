//! Failure codes, and the spellings that turn a code into text.

use std::fmt;

use crate::segment::is_segment;

#[cfg(test)]
mod tests;

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
/// use crumbkit::{Code, Spelling};
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
    /// const BAD: crumbkit::Code = crumbkit::Code::new("shelf", "books", "not-found");
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
}

/// How a tool spells its failure codes as text.
///
/// A tool passes one spelling to [`finish`](crate::finish), and its test
/// passes the same spelling to crumbkit-test's codes check. The built-in
/// spellings join the three segments of a code with a separator. A tool that
/// needs another form gives its own function to [`Spelling::custom`].
#[derive(Clone, Copy, Debug)]
pub struct Spelling {
    /// Writes a code's text to the formatter.
    write: fn(&Code, &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl Spelling {
    /// Joins the segments with `::`, as in `shelf::books::not_found`.
    pub const COLONS: Self =
        Self::custom(|code, f| write!(f, "{}::{}::{}", code.component, code.area, code.reason));

    /// Joins the segments with `.`, as in `shelf.books.not_found`.
    pub const DOTS: Self =
        Self::custom(|code, f| write!(f, "{}.{}.{}", code.component, code.area, code.reason));

    /// Joins the segments with `/`, as in `shelf/books/not_found`.
    pub const SLASHES: Self =
        Self::custom(|code, f| write!(f, "{}/{}/{}", code.component, code.area, code.reason));

    /// Returns a spelling that writes each code with `write`, which reads the
    /// segments through [`Code::component`], [`Code::area`] and
    /// [`Code::reason`].
    ///
    /// crumbkit-test's codes check fails when two of a tool's codes spell the
    /// same, or when a spelled code is empty or holds whitespace or
    /// characters outside ASCII.
    ///
    /// ```
    /// use std::fmt;
    ///
    /// use crumbkit::{Code, Spelling};
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
        Self { write }
    }

    /// Returns `code` as text in this spelling.
    ///
    /// [`finish`](crate::finish) prints a failure's code this way, and a test
    /// calls it to check how a tool's codes print.
    #[must_use]
    pub fn spell(self, code: &Code) -> String {
        Spelled {
            spelling: self,
            code,
        }
        .to_string()
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
        (self.spelling.write)(self.code, f)
    }
}
