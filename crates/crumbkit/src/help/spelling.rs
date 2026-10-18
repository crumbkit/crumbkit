//! The guide's spelling, which every code that the tool prints goes
//! through (ADR-0001, ADR-0086).

use super::Guide;
use crate::Spelling;

impl Guide {
    /// Sets how the tool spells its failure codes as text. A guide that
    /// sets none spells them with [`Spelling::COLONS`], as in
    /// `shelf::books::not_found`.
    ///
    /// [`parse`](crate::parse()) and [`Output::new`](crate::Output::new)
    /// copy the spelling into the [`Output`](crate::Output) that they
    /// return, so `finish`, `stream` and crumbkit-child's `hand_over` print
    /// every code in it, and crumbkit-test's checks read it from the guide.
    /// crumbkit's own codes print with the tool's name in place of
    /// `crumbkit`, as [`Spelling::for_tool`] describes (ADR-0079).
    ///
    /// ```
    /// use std::fmt;
    ///
    /// use crumbkit::help::Guide;
    /// use crumbkit::{Code, Spelling};
    ///
    /// fn dots(code: &Code, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///     write!(f, "{}.{}.{}", code.component(), code.area(), code.reason())
    /// }
    ///
    /// const GUIDE: Guide = Guide::new().spelling(Spelling::custom(dots));
    ///
    /// let code = Code::new("shelf", "books", "not_found");
    /// assert_eq!(GUIDE.get_spelling().spell(&code), "shelf.books.not_found");
    /// assert_eq!(Guide::new().get_spelling().spell(&code), "shelf::books::not_found");
    /// ```
    #[must_use]
    pub const fn spelling(mut self, spelling: Spelling) -> Self {
        self.spelling = spelling;
        self
    }

    /// Returns the spelling that [`Guide::spelling`] set, or
    /// [`Spelling::COLONS`] when the guide sets none. The name has clap's
    /// `get_` prefix, because `spelling` is the builder method.
    ///
    /// The spelling writes `crumbkit` in crumbkit's own codes until
    /// [`Spelling::for_tool`] names the tool, such as with
    /// [`Guide::tool_name`]. [`parse`](crate::parse) and
    /// [`Output::new`](crate::Output::new) apply the guide's tool name with
    /// [`Spelling::for_tool`], which replaces any tool name the spelling
    /// held.
    #[must_use]
    pub const fn get_spelling(&self) -> Spelling {
        self.spelling
    }
}
