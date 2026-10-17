//! The guide's settings of what a run prints on standard output: the
//! output budget (ADR-0046), the layout of a JSON document (ADR-0052) and
//! the order of its keys (ADR-0067).

use super::Guide;
use crate::output::JsonStyle;
use crate::{JsonLayout, KeyOrder};

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

    /// Sets the order of the keys of each object in every JSON document
    /// and JSON line that the tool prints on standard output (ADR-0067).
    ///
    /// Without it, each object prints its keys in the order that its value
    /// serializes them. With [`KeyOrder::Sorted`], every object at every
    /// depth prints its keys sorted: the envelope, each row, a summary, each
    /// failure, the document of the guide's
    /// [`FailureForm`](crate::form::FailureForm), each line of a stream, and
    /// the JSON lines of a text output. The order is a declaration, so it
    /// takes no reason, and crumbkit-test's opt-out report lists it under
    /// the customisations. The order changes no byte count, so the output
    /// budget cuts a document at the same rows.
    ///
    /// ```
    /// use crumbkit::KeyOrder;
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide = Guide::new().json_keys(KeyOrder::Sorted);
    /// assert_eq!(GUIDE.get_json_keys(), Some(KeyOrder::Sorted));
    /// assert_eq!(Guide::new().get_json_keys(), None);
    /// ```
    #[must_use]
    pub const fn json_keys(mut self, order: KeyOrder) -> Self {
        self.json_keys = Some(order);
        self
    }

    /// Returns the order that [`Guide::json_keys`] set, or `None` for the
    /// order that each value serializes its keys in. The name has clap's
    /// `get_` prefix.
    #[must_use]
    pub const fn get_json_keys(&self) -> Option<KeyOrder> {
        self.json_keys
    }

    /// Returns the layout that [`Guide::json_layout`] set without its
    /// reason, and the order that [`Guide::json_keys`] set.
    pub(crate) const fn json_style(&self) -> JsonStyle {
        JsonStyle {
            layout: match self.json_layout {
                Some((layout, _)) => Some(layout),
                None => None,
            },
            keys: self.json_keys,
        }
    }
}
