//! The guide's settings of what a run prints on standard output: the
//! output budget (ADR-0046) and the layout of its JSON (ADR-0052).

use super::Guide;
use crate::JsonLayout;

impl Guide {
    /// Sets the most bytes that one run prints on standard output, and the
    /// reason the tool needs that budget, which crumbkit-test's opt-out
    /// report shows beside crumbkit's default of 1 MiB (ADR-0046).
    ///
    /// A budget can be any number of bytes, as small as the one that
    /// `<TOOL>_OUTPUT_BUDGET` lowers a run to. A result that the budget
    /// refuses prints its over-budget failure whatever the failure's size,
    /// and a document marked with `Document::always_print_first` prints its
    /// first answer. With a budget of 0 bytes, every result is refused or
    /// cut, except the first answer of such a document (ADR-0081, ADR-0084).
    #[must_use]
    pub const fn output_budget(mut self, bytes: usize, reason: &'static str) -> Self {
        self.output_budget = Some((bytes, reason));
        self
    }

    /// Returns the budget and the reason that [`Guide::output_budget`] set,
    /// or `None` for crumbkit's default. The name has clap's `get_` prefix.
    #[must_use]
    pub const fn get_output_budget(&self) -> Option<(usize, &'static str)> {
        self.output_budget
    }

    /// Sets the layout of every whole JSON document that the tool prints.
    /// [`JsonLayout::Pretty`] carries the reason the tool needs it, which
    /// crumbkit-test's opt-out report shows with its cost (ADR-0052).
    ///
    /// Without it, a JSON document prints on one line. With
    /// [`JsonLayout::Pretty`], the envelope, a document from
    /// [`Envelope::Without`](crate::Envelope::Without) and the failure
    /// document print indented by two spaces, on standard output whether or
    /// not it is a terminal. A stream keeps one document per line, including
    /// a failure inside it, because JSON Lines needs it. The output budget
    /// counts the indented bytes, so an indented result fits fewer rows.
    ///
    /// ```
    /// use crumbkit::JsonLayout;
    /// use crumbkit::help::Guide;
    ///
    /// const PRETTY: JsonLayout = JsonLayout::Pretty {
    ///     reason: "Our scripts read indented JSON.",
    /// };
    /// const GUIDE: Guide = Guide::new().json_layout(PRETTY);
    /// assert_eq!(GUIDE.get_json_layout(), PRETTY);
    /// assert_eq!(Guide::new().get_json_layout(), JsonLayout::OneLine);
    /// ```
    #[must_use]
    pub const fn json_layout(mut self, layout: JsonLayout) -> Self {
        self.json_layout = layout;
        self
    }

    /// Returns the layout that [`Guide::json_layout`] set, or
    /// [`JsonLayout::OneLine`] when the guide sets none. The name has clap's
    /// `get_` prefix.
    #[must_use]
    pub const fn get_json_layout(&self) -> JsonLayout {
        self.json_layout
    }
}
