//! The guide's settings of what a run prints on standard output: the
//! output budget (ADR-0046) and the layout of a JSON document (ADR-0052).

use super::Guide;
use crate::JsonLayout;

impl Guide {
    /// Sets the most bytes that one run prints on standard output, and the
    /// reason the tool needs that budget, which crumbkit-test's opt-out
    /// report shows beside crumbkit's default of 1 MiB (ADR-0046).
    ///
    /// # Panics
    ///
    /// It panics when `bytes` is below 1024, which leaves no room for a
    /// failure document. A `const` guide reports it when the tool builds.
    #[must_use]
    pub const fn output_budget(mut self, bytes: usize, reason: &'static str) -> Self {
        let smallest = crate::output::SMALLEST_BUDGET;
        assert!(bytes >= smallest, "an output budget is at least 1024 bytes");
        self.output_budget = Some((bytes, reason));
        self
    }

    /// Returns the budget and the reason that [`Guide::output_budget`] set,
    /// or `None` for crumbkit's default. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_output_budget(&self) -> Option<(usize, &'static str)> {
        self.output_budget
    }

    /// Sets the layout of every whole JSON document that the tool prints,
    /// and the reason the tool needs it, which crumbkit-test's opt-out report
    /// shows with its cost (ADR-0052).
    ///
    /// Without it, a JSON document prints on one line. With
    /// [`JsonLayout::Pretty`], the envelope, a document from
    /// [`Document::without_envelope`](crate::Document::without_envelope),
    /// the failure document and the document of the guide's
    /// [`FailureForm`](crate::form::FailureForm) print indented by two
    /// spaces, on standard output whether or not it is a terminal. A stream
    /// keeps one document per line, including a failure inside it, because
    /// JSON Lines needs it. The output budget counts the indented bytes, so
    /// an indented result fits fewer rows.
    ///
    /// ```
    /// use crumbkit::JsonLayout;
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide =
    ///     Guide::new().json_layout(JsonLayout::Pretty, "Our scripts read indented JSON.");
    /// assert_eq!(
    ///     GUIDE.get_json_layout(),
    ///     Some((JsonLayout::Pretty, "Our scripts read indented JSON."))
    /// );
    /// ```
    #[must_use]
    pub const fn json_layout(mut self, layout: JsonLayout, reason: &'static str) -> Self {
        self.json_layout = Some((layout, reason));
        self
    }

    /// Returns the layout and the reason that [`Guide::json_layout`] set,
    /// or `None` for one line. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_json_layout(&self) -> Option<(JsonLayout, &'static str)> {
        self.json_layout
    }

    /// Returns the layout that [`Guide::json_layout`] set without its
    /// reason, or `None` for one line.
    pub(crate) const fn layout(&self) -> Option<JsonLayout> {
        match self.json_layout {
            Some((layout, _)) => Some(layout),
            None => None,
        }
    }
}
