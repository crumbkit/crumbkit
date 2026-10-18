//! The failure that a run printed, as crumbkit-test reads it back, and the
//! steps that it offers.
//!
//! crumbkit prints every failure as the `<tool>/failure/v1` document with
//! `--json`, and as the `error:`, `help:` and `code:` lines in text.
//! crumbkit-test reads either form back into a [`ReadFailure`] to follow
//! the failure's recovery. A tool that prints its failures in a document of
//! its own builds a `ReadFailure` in its crumbkit-test `FailureReader`, so
//! the checks can follow those failures too.

pub use crate::output::failure::Step;

use crate::FieldValue;

#[cfg(test)]
mod tests;

/// The failure that one run printed, as crumbkit-test reads it back from
/// the run's output: the code in the tool's spelling, the
/// steps, the recovery first and then each alternative, and the fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadFailure {
    /// The code in the tool's spelling, such as `shelf::books::not_found`.
    code: String,
    /// The recovery first, then each alternative.
    steps: Vec<Step>,
    /// Each field of the failure as a name and its value.
    fields: Vec<(String, FieldValue<'static>)>,
}

impl ReadFailure {
    /// Returns the failure with `code`, in the tool's spelling, and `steps`,
    /// the recovery first and then each alternative.
    #[must_use]
    pub const fn new(code: String, steps: Vec<Step>) -> Self {
        Self {
            code,
            steps,
            fields: Vec::new(),
        }
    }

    /// Returns the failure with `fields`, each a field's name and its value,
    /// such as `("id", FieldValue::Unsigned(7))`. A value can be any
    /// [`FieldValue`], so a reader gives back a null, list or object field as
    /// the failure held it. `check_recoveries` fills a recovery's
    /// placeholders from the text, integer and boolean fields. A failure has
    /// no fields until this method gives it some.
    ///
    /// This builder is not a `const fn`, because a `const fn` cannot drop the
    /// fields it replaces.
    #[must_use]
    pub fn with_fields(mut self, fields: Vec<(String, FieldValue<'static>)>) -> Self {
        self.fields = fields;
        self
    }

    /// Returns the code in the tool's spelling.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the steps, the recovery first and then each alternative.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Returns each field as a name and its value.
    #[must_use]
    pub fn fields(&self) -> &[(String, FieldValue<'static>)] {
        &self.fields
    }
}
