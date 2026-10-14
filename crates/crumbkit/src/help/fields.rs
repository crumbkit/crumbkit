//! The names of failure fields that a tool takes from another system
//! (ADR-0061).

use super::Guide;

impl Guide {
    /// Lists the names of failure fields that the tool takes from another
    /// system, such as `requestId` from an HTTP API, and that break
    /// crumbkit's field name rule of one or more lowercase ASCII letters,
    /// digits and underscores. A failure prints a field with a listed name
    /// in place of leaving it out, and the failure document's schema that
    /// `schemas show` serves accepts the listed names beside the rule
    /// (ADR-0061).
    ///
    /// crumbkit-test's `check_guide` reports an empty name, a name with
    /// whitespace or a control character, a name listed twice and a name
    /// that already follows the rule. A guide that lists nothing keeps the
    /// rule for every field.
    ///
    /// ```
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide = Guide::new().field_names(&["requestId"]);
    ///
    /// assert_eq!(GUIDE.get_field_names(), ["requestId"]);
    /// ```
    #[must_use]
    pub const fn field_names(mut self, names: &'static [&'static str]) -> Self {
        self.field_names = names;
        self
    }

    /// Returns the names that [`Guide::field_names`] set. The name has
    /// clap's `get_` prefix, because `field_names` is the builder method.
    #[must_use]
    pub const fn get_field_names(&self) -> &'static [&'static str] {
        self.field_names
    }
}
