//! What a [`PreparedFailure`] gives a failure form.

use super::{PreparedFailure, Step};
use crate::{Failure, FieldValue};

#[cfg(feature = "miette")]
use super::label::Label;

impl<'a> PreparedFailure<'a> {
    /// Returns the failure's code in the tool's spelling, such as
    /// `shelf::books::not_found`.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the failure's `Display`, which says what went wrong.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the `Display` of each error in the failure's `source()`
    /// chain, in order. crumbkit's document joins them with `: ` under
    /// `cause`.
    #[must_use]
    pub fn causes(&self) -> &[String] {
        &self.causes
    }

    /// Returns the step that the caller can take next: the failure's
    /// recovery with a `Run` call rendered against the tool's command tree,
    /// or the first alternative that offers a step when the recovery offers
    /// none.
    #[must_use]
    pub const fn recovery(&self) -> &Step {
        &self.step
    }

    /// Returns the failure's other alternatives that offer a step, in
    /// order, each rendered as [`PreparedFailure::recovery`] is.
    #[must_use]
    pub fn alternatives(&self) -> &[Step] {
        &self.alternatives
    }

    /// Returns the values at fault, each with its name, in the order the
    /// failure gave them. A field whose name breaks the rule of ADR-0001 or
    /// repeats an earlier name is already left out.
    pub fn fields(&self) -> impl Iterator<Item = (&str, &FieldValue<'static>)> {
        self.fields
            .iter()
            .map(|(name, value)| (name.as_str(), value))
    }

    /// Returns the page that explains the failure, when there is one.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the labels of the failure's miette diagnostic, in the order
    /// it gives them, each read against the diagnostic's source code. It is
    /// empty when the failure has no diagnostic.
    #[cfg(feature = "miette")]
    #[cfg_attr(docsrs, doc(cfg(feature = "miette")))]
    #[must_use]
    pub fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// Returns the failure that crumbkit prepared, so a form can read what
    /// crumbkit does not prepare, such as the related diagnostics of
    /// `Failure::diagnostic` with the `miette` feature.
    #[must_use]
    pub const fn failure(&self) -> &'a dyn Failure {
        self.failure
    }
}
