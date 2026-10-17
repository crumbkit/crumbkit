//! Failure forms, which let a tool print its failures in a document and a
//! text form of its own (ADR-0045).
//!
//! crumbkit prints every failure in its own form by default: the
//! `<tool>/failure/v1` document with `--json`, and the `error:`, `help:`
//! and `code:` lines in text. A tool that already publishes its own failure
//! document, which its callers read, sets a [`FailureForm`] with
//! [`Guide::failure_form`](crate::help::Guide::failure_form) and keeps that
//! document. crumbkit still prepares each failure, so the form receives the
//! code in the tool's spelling and each recovery already rendered against
//! the tool's command tree.

use std::fmt;
use std::io::{self, Write};

use schemars::Schema;

#[cfg(feature = "miette")]
pub use crate::output::failure::label::{Excerpt, Label};
pub use crate::output::failure::{PreparedFailure, Step};

use crate::{Document, FieldValue};

#[cfg(test)]
mod tests;

/// A tool's own form of a failure: the JSON document that a failure prints
/// with `--json`, its id and schema, and the text that it prints on
/// standard error.
///
/// A tool sets its form with
/// [`Guide::failure_form`](crate::help::Guide::failure_form). The form then
/// writes every failure that prints alone: the failure of a whole command
/// from [`finish`](crate::finish), each failure line of a `--json` stream,
/// a refused command line, a refused `--fields` name, and the failures of a
/// picker's source and of a child process. Its text form also prints each
/// failure of a batch or a prose result on standard error, and
/// [`FailureForm::entry`] writes each entry of their `failures` inside
/// crumbkit's envelope.
///
/// `schemas list` serves the form's document in place of
/// `<tool>/failure/v1`, and crumbkit-test's `check_schema_goldens` pins
/// its schema. A failure form is a declaration, which needs no opt-out.
/// crumbkit-test's `check_document_ids` fails only on a form whose document
/// is in the component `crumbkit`, and `check_opt_outs` lists the form under
/// the customisations (ADR-0056).
///
/// A form that keeps crumbkit's text, prints the document that the tool's
/// callers already read, and reads that document back for crumbkit-test:
///
/// ```
/// use crumbkit::form::{FailureForm, PreparedFailure, ReadFailure, Step};
/// use crumbkit::help::Guide;
/// use crumbkit::Document;
/// use crumbkit::schemars::{Schema, json_schema};
/// use serde_json::{Value, json};
///
/// /// The `{"error": ...}` document that the tool printed before it
/// /// adopted crumbkit.
/// struct ErrorForm;
///
/// impl FailureForm for ErrorForm {
///     fn document(&self) -> Document<()> {
///         Document::new("shelf", "error", 2, "The document that a command prints when it fails with --json.")
///     }
///
///     fn schema(&self) -> Schema {
///         json_schema!({
///             "type": "object",
///             "properties": {
///                 "error": {
///                     "type": "object",
///                     "properties": {
///                         "code": { "type": "string" },
///                         "message": { "type": "string" },
///                         "hint": { "type": "string" },
///                     },
///                     "required": ["code", "message"],
///                 },
///             },
///             "required": ["error"],
///         })
///     }
///
///     fn json(&self, failure: &PreparedFailure<'_>) -> Value {
///         let mut error = json!({ "code": failure.code(), "message": failure.message() });
///         let hint = match failure.recovery() {
///             Step::Run(call) => Some(format!("run `{call}`")),
///             Step::Hint(text) => Some(text.clone()),
///             _ => None,
///         };
///         if let Some(hint) = hint {
///             error["hint"] = hint.into();
///         }
///         json!({ "error": error })
///     }
///
///     // The document keeps no step that a check can follow, so the
///     // failure that crumbkit-test reads back has none.
///     fn read(&self, out: &str, _err: &str) -> Option<ReadFailure> {
///         let document: Value = serde_json::from_str(out.lines().last()?).ok()?;
///         let code = document["error"]["code"].as_str()?;
///         Some(ReadFailure::new(code.to_owned(), Vec::new()))
///     }
/// }
///
/// const GUIDE: Guide = Guide::new().failure_form(&ErrorForm);
/// ```
pub trait FailureForm {
    /// Returns the id and description of the form's document, which
    /// `schemas list` serves in place of `<tool>/failure/v1`. The id names
    /// the document in `schemas list` and `schemas show`, and the document
    /// itself need not hold it.
    fn document(&self) -> Document<()>;

    /// Returns the 2020-12 JSON Schema of the form's document. `schemas
    /// show` serves it with the document's id as its `title`.
    fn schema(&self) -> Schema;

    /// Returns the JSON document of `failure`. crumbkit writes it on one
    /// line, or in the layout and key order that the guide sets with
    /// [`Guide::json`](crate::help::Guide::json), and flushes it. A failure line of a `--json` stream stays one line in every
    /// layout, and crumbkit handles a reader that closed the pipe.
    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value;

    /// Returns the entry of `failure` in the `failures` array of a batch or
    /// a prose result, whose document is crumbkit's envelope. crumbkit still
    /// decides which failures the array holds and their order, and the
    /// failure of the rows that the output budget leaves out is an entry
    /// too.
    ///
    /// The provided method returns crumbkit's entry, the fields of the
    /// `<tool>/failure/v1` document without `schema`. A form that changes
    /// the entry also returns its schema from
    /// [`FailureForm::entry_schema`], and reads the entry back in
    /// [`FailureForm::read`]: crumbkit-test passes each entry to `read` as
    /// one line of standard output, with an empty standard error.
    fn entry(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        crate::output::failure::crumbkit_entry(failure)
    }

    /// Returns the 2020-12 JSON Schema of one entry that
    /// [`FailureForm::entry`] returns. `schemas show` serves it as the
    /// `items` of the `failures` array of every batch and prose document
    /// with crumbkit's envelope. The schema sits inside the document's
    /// schema, so it holds no `$ref` to the document's `$defs`.
    ///
    /// The provided method returns the schema of crumbkit's entry.
    fn entry_schema(&self) -> Schema {
        crate::output::failure::entry_schema()
    }

    /// Writes the text form of `failure` to `err`, which is standard error
    /// in a real run. crumbkit flushes `err` afterwards.
    ///
    /// The provided method writes crumbkit's text form, so a form that
    /// changes only the JSON document keeps crumbkit's text. A form that
    /// writes its own text escapes the control characters of the values
    /// itself, because they come from the failure and its data.
    ///
    /// # Errors
    ///
    /// Returns the error of a failed write. crumbkit leaves it unreported,
    /// because standard error is the last place to report anything.
    fn write_text(&self, mut err: &mut dyn Write, failure: &PreparedFailure<'_>) -> io::Result<()> {
        crate::output::failure::write_crumbkit_text(&mut err, failure)
    }

    /// Reads the failure that one run printed in this form, from the run's
    /// standard output `out` and standard error `err`, with or without
    /// `--json`, and returns `None` when it finds none.
    ///
    /// crumbkit never calls it. crumbkit-test's `check_twin`,
    /// `check_recoveries` and `check_examples` call it to learn a failed
    /// run's code, steps and fields, and fall back to crumbkit's own document and
    /// text when it returns `None`. A form whose document crumbkit-test
    /// cannot read needs this method for `check_recoveries` to follow its
    /// recoveries (ADR-0045).
    ///
    /// A form can write another code in place of one of crumbkit's, such as
    /// `<tool>::cli::usage`, because it receives the code from
    /// [`PreparedFailure::code`] and writes both the JSON and the text. A
    /// form that does this maps the code back to crumbkit's in `read`, so
    /// the checks still match it.
    ///
    /// crumbkit-test also passes it each entry of a batch's or a prose
    /// result's `failures` as one line of `out`, with an empty `err`, and
    /// reads an entry in crumbkit's shape when it returns `None`.
    ///
    /// The provided method returns `None`.
    fn read(&self, out: &str, err: &str) -> Option<ReadFailure> {
        let _ = (out, err);
        None
    }
}

/// The failure that one run printed in a tool's form, as
/// [`FailureForm::read`] reads it back: the code in the tool's spelling, the
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
    /// [`FieldValue`], so a form's `read` gives back a null, list or object
    /// field as the failure held it. `check_recoveries` fills a recovery's
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

/// Prints `FailureForm`, so a [`Guide`](crate::help::Guide) and an
/// [`Output`](crate::Output) that hold a form can derive `Debug`.
impl fmt::Debug for dyn FailureForm + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FailureForm")
    }
}
