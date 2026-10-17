//! What a [`PreparedFailure`] gives the rest of crumbkit.

use super::PreparedFailure;
use crate::Failure;

impl<'a> PreparedFailure<'a> {
    /// Returns the failure's code in the tool's spelling, such as
    /// `shelf::books::not_found`.
    #[cfg(feature = "tracing")]
    #[must_use]
    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    /// Returns the failure that crumbkit prepared.
    #[must_use]
    pub(crate) const fn failure(&self) -> &'a dyn Failure {
        self.failure
    }
}
