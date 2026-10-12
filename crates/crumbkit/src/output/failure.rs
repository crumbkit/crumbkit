//! How `finish` and a refusal print a failure: as text on standard error, or
//! as the `crumbkit/failure/v1` JSON document on standard output.

use std::borrow::Cow;
use std::io::{self, Write};

use clap::Command;
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::layout;
use super::table::escape_controls;
use crate::form::FailureForm;
use crate::{Document, Failure, FieldValue, JsonLayout, Recovery, Sink, Spelling};

#[cfg(feature = "miette")]
pub(crate) mod label;
mod prepared;
#[cfg(feature = "miette")]
mod related;
mod schema;
#[cfg(test)]
mod tests;

pub(crate) use self::schema::{document_schema, entry_schema};

/// The id of the JSON document that a failure prints with `--json`.
const FAILURE: Document<()> = Document::new(
    "crumbkit",
    "failure",
    1,
    "The document that a command prints on standard output when it fails with --json.",
);

/// The failure document's id and description, without its result type,
/// as `schemas list` serves it.
pub(crate) const DOCUMENT: Document<()> = FAILURE;

/// The reason a failure prints when its `Recovery::Run` call cannot be
/// rendered, because no command is set or the call does not resolve.
const UNAVAILABLE: &str = "the suggested command is not available";

/// Prints `failure` to `sink`: as text on the error writer, or with `json`
/// as one `crumbkit/failure/v1` document in `layout` on the output writer.
/// A `Recovery::Run` call renders against `command`, which builds the
/// branch that the call names.
///
/// The caller returns the exit code: 1 from `finish` for a command that
/// failed, and 2 for a refusal (ADR-0002). The flags `--long`, `--full` and
/// `--fields` do not change a failure.
pub(crate) fn print<O: Write, W: Write, R>(
    sink: &mut Sink<O, W, R>,
    json: bool,
    spelling: Spelling,
    form: Option<&dyn FailureForm>,
    layout: Option<JsonLayout>,
    failure: &dyn Failure,
    command: Option<&mut Command>,
) {
    let document = PreparedFailure::new(failure, spelling, command, &mut sink.err);
    #[cfg(feature = "tracing")]
    crate::trace::spans::failed(document.code());
    if json {
        if let Err(error) = write_json(&mut sink.out, form, layout, &document) {
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
        let _ = write_text(&mut sink.err, form, &document);
    }
}

/// A failure ready to print: its code spelled in the tool's spelling, its
/// message and causes, its recovery and alternatives rendered against the
/// tool's command tree, its fields and, with the `miette` feature, its
/// labels.
///
/// crumbkit prepares every failure that it prints once, and a
/// [`FailureForm`](crate::form::FailureForm) writes its document and its
/// text from this value, so a tool's form never spells a code or renders a
/// call itself. crumbkit's own form writes the `crumbkit/failure/v1`
/// document and its text form from the same fields.
pub struct PreparedFailure<'a> {
    /// The failure that crumbkit prepared.
    failure: &'a dyn Failure,
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
    /// The failure's fields whose names follow the rule and come first, in
    /// order.
    fields: Vec<(String, FieldValue<'static>)>,
    /// The page that explains the failure, when there is one.
    url: Option<String>,
}

/// A [`Recovery`] ready to print: a `Run` call is rendered against the tool's
/// command, and a call that cannot be rendered becomes `None` with
/// crumbkit's reason.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Step {
    /// Run this command line, such as `shelf books list`.
    Run(String),
    /// Run the same command again later.
    Retry,
    /// Follow this advice.
    Hint(String),
    /// There is no next step, for this reason.
    None(String),
}

impl<'a> PreparedFailure<'a> {
    /// Reads `failure` into the fields that `finish` prints. It spells the
    /// code with `spelling` and renders a `Recovery::Run` call against
    /// `command`, which builds the branch that the call names.
    ///
    /// It leaves out each field of the failure whose name breaks the rule or
    /// repeats an earlier name, and a debug build, such as the one a tool's
    /// tests run, writes a warning that names the field to `err`, so a test
    /// that compares standard error shows it. A debug build also writes a
    /// warning to `err` for each `Run` call of the recovery or an
    /// alternative that does not render, with the reason, so a test sees a
    /// recovery that names a command the tool lacks. A release build writes
    /// neither.
    pub(super) fn new(
        failure: &'a dyn Failure,
        spelling: Spelling,
        command: Option<&mut Command>,
        err: &mut impl Write,
    ) -> Self {
        // Every code that crumbkit prints is spelled here, so the usage log
        // records the run's code here (ADR-0029).
        #[cfg(feature = "usage-log")]
        crate::usage::failed(spelling, failure.code());
        let causes = std::iter::successors(failure.source(), |error| error.source())
            .map(ToString::to_string)
            .collect();
        let mut command = command;
        let mut unrendered = Vec::new();
        let preferred = Step::offered(failure.recovery(), command.as_deref_mut(), &mut unrendered);
        // A failure without alternatives returns an empty `Vec`, and
        // collecting an empty iterator does not allocate.
        let mut alternatives: Vec<Step> = failure
            .alternatives()
            .into_iter()
            .filter_map(|recovery| {
                Step::offered(recovery, command.as_deref_mut(), &mut unrendered).ok()
            })
            .collect();
        let step = match preferred {
            Ok(step) => step,
            Err(none) if alternatives.is_empty() => none,
            Err(_) => alternatives.remove(0),
        };
        #[cfg(feature = "miette")]
        let diagnostic = failure.diagnostic();
        let code = spelling.spell(failure.code());
        if cfg!(debug_assertions) {
            for reason in &unrendered {
                // Standard error is the last place to report anything, so a
                // failure to write there goes unreported.
                let _ = writeln!(
                    err,
                    "warning: crumbkit left out a recovery of the failure {}, because {reason}",
                    escape_controls(&code)
                );
            }
        }
        let fields = read_fields(failure, &code, err);
        Self {
            failure,
            code,
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
            fields,
            url: failure.url().map(str::to_owned),
        }
    }
}

/// Returns the fields of `failure` whose names follow the rule and come
/// first, and in a debug build writes a warning to `err` for each other
/// field, which names `code`.
///
/// A failure without fields returns an empty `Vec`, which does not allocate.
fn read_fields(
    failure: &dyn Failure,
    code: &str,
    err: &mut impl Write,
) -> Vec<(String, FieldValue<'static>)> {
    let mut fields: Vec<(String, FieldValue<'static>)> = Vec::new();
    for field in failure.fields() {
        let reason = if !field.follows_name_rule() {
            "a field name is one or more lowercase ASCII letters, digits and underscores"
        } else if fields.iter().any(|(name, _)| name == field.name()) {
            "an earlier field has that name"
        } else {
            fields.push((field.name().to_owned(), field.value().clone().into_owned()));
            continue;
        };
        if cfg!(debug_assertions) {
            // Standard error is the last place to report anything, so a
            // failure to write there goes unreported.
            let _ = writeln!(
                err,
                "warning: crumbkit left out the field {:?} of the failure {}, because {reason}",
                field.name(),
                escape_controls(code)
            );
        }
    }
    fields
}

impl Step {
    /// Reads `recovery` into a step, rendering a `Run` call against
    /// `command`. It returns `Err` with a `Step::None` when the recovery
    /// offers no step: a `Recovery::None`, or a `Run` call that cannot be
    /// rendered because no command is set or the call does not resolve. It
    /// adds the reason that a `Run` call does not render to `unrendered`.
    fn offered(
        recovery: Recovery,
        command: Option<&mut Command>,
        unrendered: &mut Vec<String>,
    ) -> Result<Self, Self> {
        match recovery {
            Recovery::Run(call) => {
                let rendered = match command {
                    Some(command) => crate::call::render(&call, command).map_err(|e| e.to_string()),
                    None => Err("no command is set".to_owned()),
                };
                rendered.map(Self::Run).map_err(|reason| {
                    unrendered.push(reason);
                    Self::None(UNAVAILABLE.to_owned())
                })
            }
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

/// Writes the text form of `document` to `err` in `form`, or in crumbkit's
/// form when `form` is `None`, then flushes `err`.
pub(super) fn write_text(
    err: &mut impl Write,
    form: Option<&dyn FailureForm>,
    document: &PreparedFailure<'_>,
) -> io::Result<()> {
    match form {
        Some(form) => {
            form.write_text(err, document)?;
            err.flush()
        }
        None => write_crumbkit_text(err, document),
    }
}

/// Writes crumbkit's text form of `document` to `err`, then flushes it, as shelf's
/// golden file [`books-show-7.stderr`][books-show-7] shows.
///
/// A `cause:` line for each cause follows the `error:` line, a `help:` line
/// for each alternative follows the recovery's `help:` line, and a `url:`
/// line follows the `code:` line when the failure has a url. With the
/// `miette` feature, an `at:` line for each label of the failure's
/// diagnostic follows the causes. When the span could be read, the lines
/// that it covers follow with one line before and after and a caret
/// underline, and labels on nearby lines share their lines, as
/// `label::write_text` describes. Each control character in a value prints
/// as its escape, so each line stays one line.
///
/// [books-show-7]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-7.stderr
pub(crate) fn write_crumbkit_text(
    err: &mut impl Write,
    document: &PreparedFailure<'_>,
) -> io::Result<()> {
    writeln!(err, "error: {}", escape_controls(&document.message))?;
    for cause in &document.causes {
        write_labelled(err, "cause:", cause)?;
    }
    #[cfg(feature = "miette")]
    label::write_text(err, &document.labels)?;
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

/// Writes `document` to `out` as JSON in `layout`, or on one line when
/// `layout` is `None`, in `form`, or as the `crumbkit/failure/v1` document
/// when `form` is `None`, then flushes `out`. A stream passes `None`, so
/// each of its failures stays one line.
pub(super) fn write_json(
    out: &mut impl Write,
    form: Option<&dyn FailureForm>,
    layout: Option<JsonLayout>,
    document: &PreparedFailure<'_>,
) -> io::Result<()> {
    match form {
        Some(form) => layout::write(out, &form.json(document), layout)?,
        None => layout::write(out, document, layout)?,
    }
    out.flush()
}

impl PreparedFailure<'_> {
    /// Writes every field of the document after `schema` into `map`: `code`,
    /// `message`, `cause` when the failure has a source, `fields` when the
    /// failure names at least one value, `labels` when the
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
        if !self.fields.is_empty() {
            map.serialize_entry("fields", &Fields(&self.fields))?;
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
/// that `PreparedFailure::serialize_fields` writes.
impl Serialize for PreparedFailure<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("schema", &format_args!("{FAILURE}"))?;
        self.serialize_fields(&mut map)?;
        map.end()
    }
}

/// A failure's fields, which serialize as one JSON object in their order.
struct Fields<'a>(&'a [(String, FieldValue<'static>)]);

impl Serialize for Fields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

/// One entry of a batch's `failures` array: the fields of the
/// `crumbkit/failure/v1` document without `schema`.
pub(super) struct Entry<'a>(pub(super) &'a PreparedFailure<'a>);

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
