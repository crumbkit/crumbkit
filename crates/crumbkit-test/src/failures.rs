//! The failures that a failed run printed, as the twin, example and recovery
//! checks read them: from crumbkit's failure document, then from
//! crumbkit's `code:` line.

use crumbkit::{FieldValue, ToolName};
use serde_json::Value;

use crate::runs;

/// The failure that one run printed, as the checks read it back from the
/// run's output: the code in the tool's spelling, the steps, the recovery
/// first and then each alternative, and the fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadFailure {
    /// The code in the tool's spelling, such as `shelf::books::not_found`.
    code: String,
    /// The recovery first, then each alternative.
    steps: Vec<Step>,
    /// Each field of the failure as a name and its value.
    fields: Vec<(String, FieldValue<'static>)>,
}

impl ReadFailure {
    /// Returns the failure with `code`, in the tool's spelling, and `steps`,
    /// the recovery first and then each alternative. It has no fields until
    /// [`ReadFailure::with_fields`] gives it some.
    pub(crate) const fn new(code: String, steps: Vec<Step>) -> Self {
        Self {
            code,
            steps,
            fields: Vec::new(),
        }
    }

    /// Returns the failure with `fields`, each a field's name and its value,
    /// such as `("id", FieldValue::Unsigned(7))`. A null, list or object
    /// field keeps the value the failure held, and `check_recoveries` fills
    /// a recovery's placeholders from the text, integer and boolean fields.
    pub(crate) fn with_fields(mut self, fields: Vec<(String, FieldValue<'static>)>) -> Self {
        self.fields = fields;
        self
    }

    /// Returns the code in the tool's spelling.
    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    /// Returns the steps, the recovery first and then each alternative.
    pub(crate) fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Returns each field as a name and its value.
    pub(crate) fn fields(&self) -> &[(String, FieldValue<'static>)] {
        &self.fields
    }
}

/// A step that a failure document offers, as the checks read it back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Run this command line, such as `shelf books list`.
    Run(String),
    /// Run this command line once with every value of the failure's list
    /// field `field` in place of the argument's placeholder.
    RunAll {
        /// The command line, with the argument's placeholder in place of the
        /// values.
        call: String,
        /// The name of the failure's field whose values the call takes.
        field: String,
    },
    /// Run the same command again later.
    Retry,
    /// Follow this advice.
    Hint(String),
    /// There is no next step, for this reason.
    None(String),
}

/// Returns each failure that a failed run printed on `stdout` and `stderr`.
/// The first of these sources that reads a failure gives them all: the
/// last failure document of `tool`, such as
/// `shelf/failure/v1`, on `stdout`, or every entry of the `failures` of the
/// last document that has any, as a batch prints them; then the last
/// `code:` line on `stderr`, which gives a failure without steps or fields.
pub(crate) fn read_failures(tool: ToolName, stdout: &[u8], stderr: &[u8]) -> Vec<ReadFailure> {
    let out = String::from_utf8_lossy(stdout);
    let err = String::from_utf8_lossy(stderr);
    let read = crumbkit_failures(tool, &out);
    if !read.is_empty() {
        return read;
    }
    err.lines()
        .rev()
        .find_map(|line| line.trim_start().strip_prefix("code:"))
        .map(|code| ReadFailure::new(code.trim().to_owned(), Vec::new()))
        .into_iter()
        .collect()
}

/// Returns the failures of the last document on `out` that is `tool`'s
/// failure document, such as `shelf/failure/v1`, or has a non-empty
/// `failures` array.
fn crumbkit_failures(tool: ToolName, out: &str) -> Vec<ReadFailure> {
    for document in runs::documents(out).iter().rev() {
        if is_failure_document(tool, document) {
            return read_document(document).into_iter().collect();
        }
        if let Some(failures) = document.get("failures").and_then(Value::as_array)
            && !failures.is_empty()
        {
            return failures.iter().filter_map(read_document).collect();
        }
    }
    Vec::new()
}

/// Reports whether `document` is `tool`'s failure document, such as
/// `shelf/failure/v1` (ADR-0079).
fn is_failure_document(tool: ToolName, document: &Value) -> bool {
    document.get("schema").and_then(Value::as_str)
        == Some(tool.schema_id("crumbkit/failure/v1").as_str())
}

/// Returns the failure of the failure document `document`: its code, its
/// steps, the recovery first and then each alternative, and its fields. It
/// returns `None` when the document has no code or no recovery.
fn read_document(document: &Value) -> Option<ReadFailure> {
    let code = document.get("code")?.as_str()?.to_owned();
    let alternatives = document
        .get("alternatives")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    let text = |step: &Value, key: &str| {
        step.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let steps: Vec<Step> = std::iter::once(document.get("recovery")?)
        .chain(alternatives)
        .map(|step| {
            let call = step.get("call").and_then(Value::as_str);
            let all = step.get("all").and_then(Value::as_str);
            match (step.get("kind").and_then(Value::as_str), call, all) {
                (Some("run"), Some(call), Some(field)) => Step::RunAll {
                    call: call.to_owned(),
                    field: field.to_owned(),
                },
                (Some("run"), Some(call), None) => Step::Run(call.to_owned()),
                (Some("run"), None, _) => Step::None(String::new()),
                (Some("retry"), ..) => Step::Retry,
                (Some("hint"), ..) => Step::Hint(text(step, "text")),
                _ => Step::None(text(step, "reason")),
            }
        })
        .collect();
    let fields = fields(document);
    Some(ReadFailure::new(code, steps).with_fields(fields))
}

/// Returns the `fields` of the failure document `document`, each a name and
/// its value, in the order the JSON object keeps them. A field whose value
/// [`FieldValue::from_json`] cannot read, such as a fraction, is left out,
/// because crumbkit never writes one: `FieldValue` has no float.
fn fields(document: &Value) -> Vec<(String, FieldValue<'static>)> {
    let Some(fields) = document.get("fields").and_then(Value::as_object) else {
        return Vec::new();
    };
    fields
        .iter()
        .filter_map(|(name, value)| {
            let value = FieldValue::from_json(value)?.into_owned();
            Some((name.clone(), value))
        })
        .collect()
}

/// Returns the text with which `value` fills a recovery's placeholder: a
/// text field as it is, and an integer or a boolean as JSON writes it. It
/// returns `None` for a null, list or object field, which fills no
/// placeholder, because an agent cannot pass JSON as one word of a command
/// line (ADR-0053).
pub(crate) fn placeholder_text(value: &FieldValue<'_>) -> Option<String> {
    match value {
        FieldValue::Text(text) => Some(text.to_string()),
        FieldValue::Integer(value) => Some(value.to_string()),
        FieldValue::Unsigned(value) => Some(value.to_string()),
        FieldValue::Boolean(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Returns the code of `failure` and its steps as one phrase, such as
/// `tool::books::not_found, whose steps are: run `tool books list`; retry`.
pub(crate) fn describe(failure: &ReadFailure) -> String {
    let steps: Vec<String> = failure
        .steps()
        .iter()
        .map(|step| match step {
            Step::Run(call) => format!("run `{call}`"),
            Step::RunAll { call, field } => format!("run `{call}` with every value of {field}"),
            Step::Retry => "retry".to_owned(),
            Step::Hint(text) => format!("hint: {text}"),
            Step::None(reason) if reason.is_empty() => "no next step".to_owned(),
            Step::None(reason) => format!("no next step: {reason}"),
        })
        .collect();
    if steps.is_empty() {
        failure.code().to_owned()
    } else {
        format!("{}, whose steps are: {}", failure.code(), steps.join("; "))
    }
}

#[cfg(test)]
mod tests;
