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
#[cfg(feature = "miette")]
mod related;
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
    /// The related diagnostics of the failure's miette diagnostic, in the
    /// order it gives them. It is empty when the failure has no diagnostic.
    #[cfg(feature = "miette")]
    related: Vec<related::Related>,
    /// The failure's recovery, with a `Run` call rendered as text, or the
    /// first alternative that offers a step when the recovery offers none.
    step: Step,
    /// The failure's other alternatives that offer a step, in order.
    alternatives: Vec<Step>,
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
        let mut command = command;
        let preferred = Step::offered(failure.recovery(), command.as_deref_mut());
        // A failure without alternatives returns an empty `Vec`, and
        // collecting an empty iterator does not allocate.
        let mut alternatives: Vec<Step> = failure
            .alternatives()
            .into_iter()
            .filter_map(|recovery| Step::offered(recovery, command.as_deref_mut()).ok())
            .collect();
        let step = match preferred {
            Ok(step) => step,
            Err(none) if alternatives.is_empty() => none,
            Err(_) => alternatives.remove(0),
        };
        #[cfg(feature = "miette")]
        let diagnostic = failure.diagnostic();
        Self {
            code: spelling.spell(failure.code()),
            message: failure.to_string(),
            causes,
            // A failure without a diagnostic costs one `None` check, and an
            // empty `Vec` does not allocate.
            #[cfg(feature = "miette")]
            labels: diagnostic.map_or_else(Vec::new, |diagnostic| {
                label::read(diagnostic, diagnostic.source_code())
            }),
            #[cfg(feature = "miette")]
            related: diagnostic.map_or_else(Vec::new, |diagnostic| {
                related::read(diagnostic, diagnostic.source_code())
            }),
            step,
            alternatives,
            url: failure.url().map(str::to_owned),
        }
    }

    /// Returns the failure's code in the tool's spelling.
    #[cfg(feature = "tracing")]
    pub(super) fn code(&self) -> &str {
        &self.code
    }
}

impl Step {
    /// Reads `recovery` into a step, rendering a `Run` call against
    /// `command`. It returns `Err` with a `Step::None` when the recovery
    /// offers no step: a `Recovery::None`, or a `Run` call that cannot be
    /// rendered because no command is set or the call does not resolve.
    fn offered(recovery: Recovery, command: Option<&mut Command>) -> Result<Self, Self> {
        match recovery {
            Recovery::Run(call) => command
                .and_then(|command| crate::call::render(&call, command).ok())
                .map(Self::Run)
                .ok_or_else(|| Self::None(UNAVAILABLE.to_owned())),
            Recovery::Retry => Ok(Self::Retry),
            Recovery::Hint(text) => Ok(Self::Hint(text)),
            Recovery::None { reason } => Err(Self::None(reason)),
            // `Recovery` is `#[non_exhaustive]` and lives in crumbkit-core, so
            // the compiler requires this arm. crumbkit-core releases with
            // crumbkit (ADR-0012), so a new variant gets its own arm in the
            // release that adds it, and this arm matches nothing until then.
            _ => Err(Self::None(UNAVAILABLE.to_owned())),
        }
    }

    /// Returns the text of the step's `help:` line.
    fn help(&self) -> Cow<'_, str> {
        match self {
            Self::Run(call) => Cow::Owned(format!("run `{call}`")),
            Self::Retry => Cow::Borrowed("run the same command again later"),
            Self::Hint(text) => Cow::Borrowed(text),
            Self::None(reason) => Cow::Owned(format!("no next step: {reason}")),
        }
    }
}

/// Writes the text form of `document` to `err`, then flushes it, as shelf's
/// golden file [`books-show-7.stderr`][books-show-7] shows.
///
/// A `cause:` line for each cause follows the `error:` line, a `help:` line
/// for each alternative follows the recovery's `help:` line, and a `url:`
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
    write_labelled(err, "help:", &document.step.help())?;
    for alternative in &document.alternatives {
        write_labelled(err, "help:", &alternative.help())?;
    }
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
    /// failure has a diagnostic with at least one label, `related` when that
    /// diagnostic has related diagnostics, `recovery`,
    /// `alternatives` when at least one alternative offers a step, and `url`
    /// when the failure has one. The failure document and an entry of
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
        #[cfg(feature = "miette")]
        if !self.related.is_empty() {
            map.serialize_entry("related", &self.related)?;
        }
        map.serialize_entry("recovery", &self.step)?;
        if !self.alternatives.is_empty() {
            map.serialize_entry("alternatives", &self.alternatives)?;
        }
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
                "related": related_schema(),
                "recovery": step_schema("What the caller can do next, tagged by kind."),
                "alternatives": {
                    "description": "Other repairs the caller can try, in order of preference, each in the shape of recovery. It is present only when the failure offers at least one.",
                    "type": "array",
                    "items": step_schema("A repair the caller can try instead, tagged by kind."),
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

/// Describes one step of the `crumbkit/failure/v1` document, tagged by
/// `kind`, as `recovery` and each entry of `alternatives` hold it, with
/// `description` as its description.
fn step_schema(description: &str) -> Schema {
    json_schema!({
        "description": description,
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
    })
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
                "excerpt": {
                    "description": "The lines of the source that the span covers, with one line before and one after where the source has them. It is present only when the span could be read.",
                    "type": "object",
                    "properties": {
                        "line": {
                            "description": "The 1-based number of the first line of the excerpt.",
                            "type": "integer",
                            "minimum": 1,
                        },
                        "text": {
                            "description": "The lines, joined by the line breaks of the source, without a line break at the end.",
                            "type": "string",
                        },
                    },
                    "required": ["line", "text"],
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

/// Describes the `related` array of the `crumbkit/failure/v1` document. Each
/// entry has the `$anchor` `related`, and its own `related` array refers to
/// the entry's schema through it, because related diagnostics nest. The
/// schema lists the array whether or not the `miette` feature is on, as it
/// does `labels`.
fn related_schema() -> Schema {
    json_schema!({
        "description": "The diagnostics that the failure's diagnostic relates to it, in the order it gives them. It is present only when there is at least one.",
        "type": "array",
        "items": {
            "$anchor": "related",
            "type": "object",
            "properties": {
                "message": {
                    "description": "What the related diagnostic says.",
                    "type": "string",
                },
                "code": {
                    "description": "The related diagnostic's own code, which is free text. It is present only when the diagnostic has one.",
                    "type": "string",
                },
                "severity": {
                    "description": "How severe the related diagnostic is. A diagnostic that gives no severity is an error.",
                    "enum": ["error", "warning", "advice"],
                },
                "cause": {
                    "description": "The errors below the related diagnostic, joined with \": \". It is present only when there is one.",
                    "type": "string",
                },
                "labels": labels_schema(),
                "help": {
                    "description": "The related diagnostic's advice. It is present only when it gives some.",
                    "type": "string",
                },
                "url": {
                    "description": "A page that explains the related diagnostic. It is present only when it has one.",
                    "type": "string",
                },
                "related": {
                    "description": "The diagnostics related to this one, in the same shape, to at most eight levels below the failure.",
                    "type": "array",
                    "items": { "$ref": "#related" },
                },
            },
            "required": ["message", "severity"],
        },
    })
}
