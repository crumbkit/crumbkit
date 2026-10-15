//! The failures that a failed run printed, as the twin, example and recovery
//! checks read them: through the tool's failure form or failure reader,
//! then from crumbkit's failure document, then from crumbkit's `code:` line.

use std::fmt;

use crumbkit::FieldValue;
use crumbkit::form::{FailureForm, ReadFailure, Step};
use crumbkit::help::Guide;
use serde_json::Value;

use crate::examples::Examples;
use crate::runs;

/// Reads the failures that one failed run of a tool printed, for a tool
/// that prints its failures in a shape of its own without a
/// [`FailureForm`]. A tool sets its reader with
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
/// error. A guide that sets a failure form with `Guide::failure_form` reads
/// its failures through [`FailureForm::read`], and a check fails when the
/// guide sets a form and the examples also set a reader.
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

/// The tool's own way to read a failed run: the guide's failure form or
/// the examples' failure reader.
#[derive(Clone, Copy)]
pub(crate) enum Reader<'a> {
    /// The failure form that the guide sets.
    Form(&'a dyn FailureForm),
    /// The failure reader that the examples set.
    Own(&'a dyn FailureReader),
}

impl Reader<'_> {
    /// Returns each failure that this reader reads from `out` and `err`.
    fn read(self, out: &str, err: &str) -> Vec<ReadFailure> {
        match self {
            Self::Form(form) => form.read(out, err).into_iter().collect(),
            Self::Own(reader) => reader.read(out, err),
        }
    }

    /// Returns the reader's name and the way a tool makes it read a
    /// failure, which a check's problem names.
    pub(crate) fn name_and_remedy(self) -> (String, &'static str) {
        match self {
            Self::Form(form) => (
                format!("the failure form {}", form.document()),
                "The tool implements FailureForm::read",
            ),
            Self::Own(_) => (
                "the tool's FailureReader".to_owned(),
                "The tool's FailureReader reads the failure",
            ),
        }
    }
}

/// Returns the guide's failure form or the examples' failure reader, or
/// `None` when neither is set, or the problem when both are set.
pub(crate) fn reader<'a>(
    guide: &Guide,
    examples: &Examples<'a>,
) -> Result<Option<Reader<'a>>, String> {
    match (guide.get_failure_form(), examples.reader()) {
        (Some(_), Some(_)) => Err(
            "the guide sets a failure form with Guide::failure_form and the examples set a failure reader with Examples::failure_reader, and a tool reads its failures through only one of them"
                .to_owned(),
        ),
        (Some(form), None) => Ok(Some(Reader::Form(form))),
        (None, Some(reader)) => Ok(Some(Reader::Own(reader))),
        (None, None) => Ok(None),
    }
}

/// Returns each failure that a failed run printed on `stdout` and `stderr`.
/// The first of these sources that reads a failure gives them all:
/// `reader`; then the last `crumbkit/failure/v1` document on `stdout`, or
/// every entry of the `failures` of the last document that has any, as a
/// batch prints them, or every failure document of the last array that
/// holds any, at the top or under a key of the document, as a batch in
/// input order prints them; then the last `code:`
/// line on `stderr`, which gives a
/// failure without steps or fields.
///
/// When `reader` is a failure form, each entry of `failures` is read
/// through [`FailureForm::read`] as one line of standard output with an
/// empty standard error, and an entry that the form cannot read is read in
/// crumbkit's shape.
pub(crate) fn read_failures(
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
    let form = match reader {
        Some(Reader::Form(form)) => Some(form),
        _ => None,
    };
    let read = crumbkit_failures(&out, form);
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

/// Returns the failures of the last document on `out` that is a
/// `crumbkit/failure/v1` document, has a non-empty `failures` array, or is
/// an array that holds at least one failure document, as a batch that
/// prints its failures in input order prints it, at the top or under any
/// key of an object, as a document with keys from `Document::with_keys`
/// holds it. A `form` reads each entry
/// of `failures` and each item of such an array first. An item that the
/// form does not read is a failure only when it is a
/// `crumbkit/failure/v1` document, so a row is never read as one.
fn crumbkit_failures(out: &str, form: Option<&dyn FailureForm>) -> Vec<ReadFailure> {
    let through_form = |item: &Value| form.and_then(|form| form.read(&item.to_string(), ""));
    let in_order = |items: &[Value]| -> Vec<ReadFailure> {
        items
            .iter()
            .filter_map(|item| {
                through_form(item)
                    .or_else(|| is_failure_document(item).then(|| read_document(item))?)
            })
            .collect()
    };
    for document in runs::documents(out).iter().rev() {
        if is_failure_document(document) {
            return read_document(document).into_iter().collect();
        }
        if let Some(items) = document.as_array() {
            let read = in_order(items);
            if !read.is_empty() {
                return read;
            }
        }
        if let Some(failures) = document.get("failures").and_then(Value::as_array)
            && !failures.is_empty()
        {
            return failures
                .iter()
                .filter_map(|entry| through_form(entry).or_else(|| read_document(entry)))
                .collect();
        }
        // A document with keys that the tool declares holds a batch in
        // input order under its data key, which the reader does not know.
        let arrays = document
            .as_object()
            .into_iter()
            .flat_map(|object| object.values())
            .filter_map(Value::as_array);
        for items in arrays {
            let read = in_order(items);
            if !read.is_empty() {
                return read;
            }
        }
    }
    Vec::new()
}

/// Reports whether `document` is a `crumbkit/failure/v1` document.
fn is_failure_document(document: &Value) -> bool {
    document.get("schema").and_then(Value::as_str) == Some("crumbkit/failure/v1")
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
