//! The guide's settings of what a run prints on standard output: the
//! output budget (ADR-0046) and the style of its JSON (ADR-0073).

use super::Guide;
use crate::JsonStyle;

impl Guide {
    /// Sets the most bytes that one run prints on standard output, and the
    /// reason the tool needs that budget, which crumbkit-test's opt-out
    /// report shows beside crumbkit's default of 1 MiB (ADR-0046).
    ///
    /// A budget can be as small as 1 byte, so a tool's tests can show a cut
    /// result with a few short rows. A result that the budget refuses
    /// prints its over-budget failure whatever the failure's size.
    ///
    /// # Panics
    ///
    /// It panics when `bytes` is 0, because a run with a budget of 0 bytes
    /// could print no answer. A `const` guide reports it when the tool
    /// builds.
    #[must_use]
    pub const fn output_budget(mut self, bytes: usize, reason: &'static str) -> Self {
        assert!(bytes >= 1, "an output budget is at least 1 byte");
        self.output_budget = Some((bytes, reason));
        self
    }

    /// Returns the budget and the reason that [`Guide::output_budget`] set,
    /// or `None` for crumbkit's default. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_output_budget(&self) -> Option<(usize, &'static str)> {
        self.output_budget
    }

    /// Sets how the tool writes JSON on standard output: the layout of a
    /// whole document and the order of the keys of each object (ADR-0073).
    ///
    /// Without it, a JSON document prints on one line, and each object
    /// prints its keys in the order that its value serializes them.
    ///
    /// With [`JsonLayout::Pretty`](crate::JsonLayout::Pretty), the
    /// envelope, a document from
    /// [`Envelope::Without`](crate::Envelope::Without), the failure
    /// document and the document of the guide's
    /// [`FailureForm`](crate::form::FailureForm) print indented by two
    /// spaces, on standard output whether or not it is a terminal. A stream
    /// keeps one document per line, including a failure inside it, because
    /// JSON Lines needs it. The output budget counts the indented bytes, so
    /// an indented result fits fewer rows. crumbkit-test's opt-out report
    /// shows the layout's reason with its cost (ADR-0052).
    ///
    /// With `sorted_keys`, every object at every depth prints its keys
    /// sorted: the envelope, each row, a summary, each failure, the
    /// document of the guide's [`FailureForm`](crate::form::FailureForm),
    /// each line of a stream, and each JSON line of a text output. The
    /// order is a declaration, so it takes no reason, and crumbkit-test's
    /// opt-out report lists it under the customisations. The order changes
    /// no byte count, so the output budget cuts a document at the same
    /// rows (ADR-0067).
    ///
    /// ```
    /// use crumbkit::JsonStyle;
    /// use crumbkit::help::Guide;
    ///
    /// const STYLE: JsonStyle = JsonStyle::new().pretty("Our scripts read indented JSON.");
    /// const GUIDE: Guide = Guide::new().json(STYLE);
    /// assert_eq!(GUIDE.get_json(), STYLE);
    /// assert_eq!(Guide::new().get_json(), JsonStyle::new());
    /// ```
    #[must_use]
    pub const fn json(mut self, style: JsonStyle) -> Self {
        self.json = style;
        self
    }

    /// Returns the style that [`Guide::json`] set, or [`JsonStyle::new`]
    /// when the guide sets none. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_json(&self) -> JsonStyle {
        self.json
    }
}
