//! How `finish` and a refusal print a failure: as text on standard error, or
//! as the `crumbkit/failure/v1` JSON document on standard output.

use std::borrow::Cow;
use std::io::{self, Write};

use clap::Command;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::table::escape_controls;
use crate::{Document, Failure, Recovery, Sink, Spelling};

#[cfg(feature = "miette")]
mod label;
#[cfg(test)]
mod tests;

/// The id of the JSON document that a failure prints with `--json`.
const FAILURE: Document<FailureDocument> = Document::new(
    "crumbkit",
    "failure",
    1,
    "The document that a command prints on standard output when it fails with --json.",
);

/// The failure document's id and description, without its result type,
/// as `schemas list` serves it.
pub(crate) const DOCUMENT: Document<()> = FAILURE.untyped();

/// The reason a failure prints when its `Recovery::Run` call cannot be
/// rendered, because no command is set or the call does not resolve.
const UNAVAILABLE: &str = "the suggested command is not available";

/// Prints `failure` to `sink`: as text on the error writer, or with `json`
/// as one `crumbkit/failure/v1` document on the output writer. A
/// `Recovery::Run` call renders against `command`, which builds the branch
/// that the call names.
///
/// The caller returns the exit code: 1 from `finish` for a command that
/// failed, and 2 for a refusal (ADR-0002). The flags `--long`, `--full` and
/// `--fields` do not change a failure.
pub(crate) fn print<O: Write, W: Write>(
    sink: &mut Sink<O, W>,
    json: bool,
    spelling: Spelling,
    failure: &dyn Failure,
    command: Option<&mut Command>,
) {
    let document = FailureDocument::new(failure, spelling, command);
    #[cfg(feature = "tracing")]
    crate::trace::spans::failed(document.code());
    if json {
        if let Err(error) = write_json(&mut sink.out, &document) {
            // A reader that closed the pipe early has stopped reading, so
            // only another write error is reported. The process is already
            // failing, and a failed report has nowhere else to go.
            if error.kind() != io::ErrorKind::BrokenPipe {
                let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            }
        }
    } else {
        // Standard error is the last place to report anything, so a failure
        // to write there goes unreported.
        let _ = write_text(&mut sink.err, &document);
    }
}

/// A failure as `finish` prints it. It holds the fields of the
/// `crumbkit/failure/v1` document, and the text form prints the same fields.
/// A batch prints each of its failures from one of these as well.
pub(super) struct FailureDocument {
    /// The failure's code in the tool's spelling.
    code: String,
    /// The failure's `Display`.
    message: String,
    /// The `Display` of each error in the failure's `source()` chain, in
    /// order.
    causes: Vec<String>,
    /// The labels of the failure's miette diagnostic, in the order it gives
    /// them. It is empty when the failure has no diagnostic.
    #[cfg(feature = "miette")]
    labels: Vec<label::Label>,
    /// The failure's recovery, with a `Run` call rendered as text.
    step: Step,
    /// The page that explains the failure, when there is one.
    url: Option<String>,
}

/// A [`Recovery`] ready to print: a `Run` call is rendered against the tool's
/// command, and a call that cannot be rendered becomes `None`.
enum Step {
    /// Run this command line.
    Run(String),
    /// Run the same command again later.
    Retry,
    /// Follow this advice.
    Hint(String),
    /// There is no next step, for this reason.
    None(String),
}

impl FailureDocument {
    /// Reads `failure` into the fields that `finish` prints. It spells the
    /// code with `spelling` and renders a `Recovery::Run` call against
    /// `command`, which builds the branch that the call names.
    pub(super) fn new(
        failure: &dyn Failure,
        spelling: Spelling,
        command: Option<&mut Command>,
    ) -> Self {
        // Every code that crumbkit prints is spelled here, so the usage log
        // records the run's code here (ADR-0029).
        #[cfg(feature = "usage-log")]
        crate::usage::failed(spelling, failure.code());
        let causes = std::iter::successors(failure.source(), |error| error.source())
            .map(ToString::to_string)
            .collect();
        let step = match failure.recovery() {
            Recovery::Run(call) => command
                .and_then(|command| crate::call::render(&call, command).ok())
                .map_or_else(|| Step::None(UNAVAILABLE.to_owned()), Step::Run),
            Recovery::Retry => Step::Retry,
            Recovery::Hint(text) => Step::Hint(text),
            Recovery::None { reason } => Step::None(reason),
            // `Recovery` is `#[non_exhaustive]` and lives in crumbkit-core, so
            // the compiler requires this arm. crumbkit-core releases with
            // crumbkit (ADR-0012), so a new variant gets its own arm in the
            // release that adds it, and this arm matches nothing until then.
            _ => Step::None(UNAVAILABLE.to_owned()),
        };
        Self {
            code: spelling.spell(failure.code()),
            message: failure.to_string(),
            causes,
            // A failure without a diagnostic costs one `None` check, and an
            // empty `Vec` does not allocate.
            #[cfg(feature = "miette")]
            labels: failure.diagnostic().map_or_else(Vec::new, label::read),
            step,
            url: failure.url().map(str::to_owned),
        }
    }

    /// Returns the failure's code in the tool's spelling.
    #[cfg(feature = "tracing")]
    pub(super) fn code(&self) -> &str {
        &self.code
    }

    /// Returns the text of the `help:` line.
    fn help(&self) -> Cow<'_, str> {
        match &self.step {
            Step::Run(call) => Cow::Owned(format!("run `{call}`")),
            Step::Retry => Cow::Borrowed("run the same command again later"),
            Step::Hint(text) => Cow::Borrowed(text),
            Step::None(reason) => Cow::Owned(format!("no next step: {reason}")),
        }
    }
}

/// Writes the text form of `document` to `err`, then flushes it, as shelf's
/// golden file [`books-show-7.stderr`][books-show-7] shows.
///
/// A `cause:` line for each cause follows the `error:` line, and a `url:`
/// line follows the `code:` line when the failure has a url. With the
/// `miette` feature, an `at:` line for each label of the failure's
/// diagnostic follows the causes, each with its source line and an
/// underline when the span could be read. Each control character in a value
/// prints as its escape, so each line stays one line.
///
/// [books-show-7]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-7.stderr
pub(super) fn write_text(err: &mut impl Write, document: &FailureDocument) -> io::Result<()> {
    writeln!(err, "error: {}", escape_controls(&document.message))?;
    for cause in &document.causes {
        write_labelled(err, "cause:", cause)?;
    }
    #[cfg(feature = "miette")]
    for label in &document.labels {
        label.write_text(err)?;
    }
    write_labelled(err, "help:", &document.help())?;
    write_labelled(err, "code:", &document.code)?;
    if let Some(url) = &document.url {
        write_labelled(err, "url:", url)?;
    }
    err.flush()
}

/// Writes one indented line with `label` padded to the width of `cause:`,
/// so the values line up.
fn write_labelled(err: &mut impl Write, label: &str, value: &str) -> io::Result<()> {
    writeln!(err, "  {label:<6} {}", escape_controls(value))
}

/// Writes `document` to `out` as JSON on one line, then flushes `out`.
fn write_json(out: &mut impl Write, document: &FailureDocument) -> io::Result<()> {
    serde_json::to_writer(&mut *out, document)?;
    out.write_all(b"\n")?;
    out.flush()
}

impl FailureDocument {
    /// Writes every field of the document after `schema` into `map`: `code`,
    /// `message`, `cause` when the failure has a source, `labels` when the
    /// failure has a diagnostic with at least one label, `recovery`, and
    /// `url` when the failure has one. The failure document and an entry of
    /// a batch's `failures` both write their fields here, so the two differ
    /// only in `schema`.
    ///
    /// `cause` is one string, the causes joined with `: `, as a log line
    /// shows a chain of errors.
    fn serialize_fields<M: SerializeMap>(&self, map: &mut M) -> Result<(), M::Error> {
        map.serialize_entry("code", &self.code)?;
        map.serialize_entry("message", &self.message)?;
        if !self.causes.is_empty() {
            map.serialize_entry("cause", &self.causes.join(": "))?;
        }
        #[cfg(feature = "miette")]
        if !self.labels.is_empty() {
            map.serialize_entry("labels", &self.labels)?;
        }
        map.serialize_entry("recovery", &self.step)?;
        if let Some(url) = &self.url {
            map.serialize_entry("url", url)?;
        }
        Ok(())
    }
}

/// Serializes the `crumbkit/failure/v1` document: `schema`, then the fields
/// that [`FailureDocument::serialize_fields`] writes.
impl Serialize for FailureDocument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("schema", &format_args!("{FAILURE}"))?;
        self.serialize_fields(&mut map)?;
        map.end()
    }
}

/// One entry of a batch's `failures` array: the fields of the
/// `crumbkit/failure/v1` document without `schema`.
pub(super) struct Entry<'a>(pub(super) &'a FailureDocument);

impl Serialize for Entry<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        self.0.serialize_fields(&mut map)?;
        map.end()
    }
}

/// Serializes the step as an object tagged by `kind`, such as
/// `{"kind":"run","call":"shelf books list"}` or `{"kind":"retry"}`.
impl Serialize for Step {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (kind, field) = match self {
            Self::Run(call) => ("run", Some(("call", call))),
            Self::Retry => ("retry", None),
            Self::Hint(text) => ("hint", Some(("text", text))),
            Self::None(reason) => ("none", Some(("reason", reason))),
        };
        let mut map = serializer.serialize_map(Some(1 + usize::from(field.is_some())))?;
        map.serialize_entry("kind", kind)?;
        if let Some((name, value)) = field {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

/// Describes the `crumbkit/failure/v1` document as a 2020-12 JSON Schema.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature. `crates/crumbkit/schemas/failure-v1.json` holds the schema, and a
/// test fails when the two differ (ADR-0007).
impl JsonSchema for FailureDocument {
    fn schema_name() -> Cow<'static, str> {
        "FailureDocument".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit/failure/v1".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "The document that a command prints on standard output when it fails with --json.",
            "type": "object",
            "properties": {
                "schema": {
                    "description": "The id of this document.",
                    "const": "crumbkit/failure/v1",
                },
                "code": {
                    "description": "The failure's stable code in the tool's spelling, such as shelf::books::not_found. It is one or more printable ASCII characters without spaces, from ! to ~.",
                    "type": "string",
                    "pattern": "^[!-~]+$",
                },
                "message": {
                    "description": "What went wrong.",
                    "type": "string",
                },
                "cause": {
                    "description": "The errors that led to the failure, joined with \": \". It is present only when the failure has a cause.",
                    "type": "string",
                },
                "labels": labels_schema(),
                "recovery": {
                    "description": "What the caller can do next, tagged by kind.",
                    "oneOf": [
                        {
                            "description": "Run this command.",
                            "type": "object",
                            "properties": {
                                "kind": { "const": "run" },
                                "call": { "type": "string" },
                            },
                            "required": ["kind", "call"],
                        },
                        {
                            "description": "Run the same command again later.",
                            "type": "object",
                            "properties": {
                                "kind": { "const": "retry" },
                            },
                            "required": ["kind"],
                        },
                        {
                            "description": "Follow this advice.",
                            "type": "object",
                            "properties": {
                                "kind": { "const": "hint" },
                                "text": { "type": "string" },
                            },
                            "required": ["kind", "text"],
                        },
                        {
                            "description": "There is no next step, for this reason.",
                            "type": "object",
                            "properties": {
                                "kind": { "const": "none" },
                                "reason": { "type": "string" },
                            },
                            "required": ["kind", "reason"],
                        },
                    ],
                },
                "url": {
                    "description": "A page that explains the failure. It is present only when the failure has one.",
                    "type": "string",
                },
            },
            "required": ["schema", "code", "message", "recovery"],
        })
    }
}

/// Returns the JSON Schema of the `crumbkit/failure/v1` document, with the
/// document's id as its `title`, as `schemas show` serves it and
/// `crates/crumbkit/schemas/failure-v1.json` holds it.
pub(crate) fn document_schema() -> Schema {
    let mut schema = schemars::schema_for!(FailureDocument);
    schema.insert("title".to_owned(), FAILURE.to_string().into());
    schema
}

/// Returns the JSON Schema of one entry of a batch's `failures` array: the
/// schema of the `crumbkit/failure/v1` document without its `schema`
/// property, as [`Entry`] writes it.
pub(crate) fn entry_schema() -> Schema {
    let mut schema = FailureDocument::json_schema(&mut SchemaGenerator::default());
    schema.insert(
        "description".to_owned(),
        "A failure of some of the ids: the fields of the crumbkit/failure/v1 document without schema."
            .into(),
    );
    if let Some(properties) = schema
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    {
        properties.remove("schema");
    }
    if let Some(required) = schema
        .get_mut("required")
        .and_then(serde_json::Value::as_array_mut)
    {
        required.retain(|name| name != "schema");
    }
    schema
}

/// Describes the `labels` array of the `crumbkit/failure/v1` document. The
/// schema lists it whether or not the `miette` feature is on, so there is
/// one checked-in schema file, and readers ignore a field they do not know
/// (ADR-0007).
fn labels_schema() -> Schema {
    json_schema!({
        "description": "The places in a source, such as a file, that the failure points at, in the order the failure gives them. It is present only when the failure has at least one label.",
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "offset": {
                    "description": "The byte offset of the start of the span from the start of the source.",
                    "type": "integer",
                    "minimum": 0,
                },
                "length": {
                    "description": "The length of the span in bytes.",
                    "type": "integer",
                    "minimum": 0,
                },
                "name": {
                    "description": "The name of the source, such as a file name. It is present only when the span could be read and the source has a name.",
                    "type": "string",
                },
                "line": {
                    "description": "The 1-based line that holds the start of the span. It is present only when the span could be read.",
                    "type": "integer",
                    "minimum": 1,
                },
                "column": {
                    "description": "The 1-based column of the start of the span, counted in characters of its line. It is present only when the span could be read.",
                    "type": "integer",
                    "minimum": 1,
                },
                "text": {
                    "description": "What the label says about the span. It is present only when the label has text.",
                    "type": "string",
                },
            },
            "required": ["offset", "length"],
        },
    })
}
