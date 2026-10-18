//! The failures that a failed run printed, as the twin, example and recovery
//! checks read them: through the tool's failure reader, then from
//! crumbkit's failure document, then from crumbkit's `code:` line.

use std::fmt;

use crumbkit::form::{ReadFailure, Step};
use crumbkit::{FieldValue, ToolName};
use serde_json::Value;

use crate::examples::Examples;
use crate::runs;

/// Reads the failures that one failed run of a tool printed, for a tool
/// that prints its failures in a shape of its own. A tool sets its reader with
/// [`Examples::failure_reader`], and [`check_twin`](crate::check_twin),
/// [`check_examples`](crate::check_examples) and
/// [`check_recoveries`](crate::check_recoveries) then read each failed run
/// through it (ADR-0045).
///
/// `read` receives the run's standard output `out` and standard error
/// `err`, and returns each failure it finds, the first failure first, or
/// no failure when it finds none. Each failure holds its code in the tool's
/// spelling, its steps, the recovery first and then each alternative, and
/// its fields, which fill the placeholders of a recovery's call. A reader
/// maps a code that the tool writes in place of one of crumbkit's, such as
/// its own usage code in place of `crumbkit::cli::usage`, back to
/// crumbkit's code, so the checks match it.
///
/// When the reader returns no failure, the checks read crumbkit's failure
/// document on standard output and then the last `code:` line on standard
/// error.
///
/// [`check_recoveries`](crate::check_recoveries) adds `--json` to every
/// run, so a tool whose failures a reader reads still accepts `--json` on
/// every command that a chain runs.
///
/// ```
/// use crumbkit::form::{ReadFailure, Step};
/// use crumbkit_test::{Examples, FailureReader};
///
/// /// Reads a last line such as `ERR shelf::books::not_found`.
/// struct ErrLine;
///
/// impl FailureReader for ErrLine {
///     fn read(&self, _out: &str, err: &str) -> Vec<ReadFailure> {
///         err.lines()
///             .last()
///             .and_then(|line| line.strip_prefix("ERR "))
///             .map(|code| ReadFailure::new(code.to_owned(), vec![Step::Retry]))
///             .into_iter()
///             .collect()
///     }
/// }
///
/// const EXAMPLES: Examples<'_> =
///     Examples::new("/path/to/shelf", "tests/golden/examples").failure_reader(&ErrLine);
/// ```
pub trait FailureReader {
    /// Returns each failure that the run printed on standard output `out`
    /// and standard error `err`, the first failure first, or no failure
    /// when it finds none.
    fn read(&self, out: &str, err: &str) -> Vec<ReadFailure>;
}

/// Prints `FailureReader`, so [`Examples`], which can hold a reader, can
/// derive `Debug`.
impl fmt::Debug for dyn FailureReader + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FailureReader")
    }
}

/// The tool's own way to read a failed run: the examples' failure reader.
#[derive(Clone, Copy)]
pub(crate) struct Reader<'a>(pub(crate) &'a dyn FailureReader);

impl Reader<'_> {
    /// Returns each failure that this reader reads from `out` and `err`.
    fn read(self, out: &str, err: &str) -> Vec<ReadFailure> {
        self.0.read(out, err)
    }
}

/// Returns the examples' failure reader, or `None` when they set none.
pub(crate) fn reader<'a>(examples: &Examples<'a>) -> Option<Reader<'a>> {
    examples.reader().map(Reader)
}

/// Returns each failure that a failed run printed on `stdout` and `stderr`.
/// The first of these sources that reads a failure gives them all:
/// `reader`; then the last failure document of `tool`, such as
/// `shelf/failure/v1`, on `stdout`, or every entry of the `failures` of the
/// last document that has any, as a batch prints them; then the last
/// `code:` line on `stderr`, which gives a failure without steps or fields.
pub(crate) fn read_failures(
    tool: ToolName,
    reader: Option<Reader<'_>>,
    stdout: &[u8],
    stderr: &[u8],
) -> Vec<ReadFailure> {
    let out = String::from_utf8_lossy(stdout);
    let err = String::from_utf8_lossy(stderr);
    if let Some(reader) = reader {
        let read = reader.read(&out, &err);
        if !read.is_empty() {
            return read;
        }
    }
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
            _ => "a step that crumbkit-test cannot name".to_owned(),
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
