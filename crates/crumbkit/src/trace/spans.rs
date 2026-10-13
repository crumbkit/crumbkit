//! The `tracing` spans of `parse` and `finish`, which the `tracing` feature
//! adds (ADR-0030).
//!
//! The field names follow OpenTelemetry's semantic conventions for CLI
//! programs where one exists: `process.exit.code` and `error.type`.
//! `trace_id` is the name that OpenTelemetry gives the trace id in log
//! formats other than OTLP, so a subscriber that writes JSON lines puts the
//! id where OpenTelemetry tools look for it. `crumbkit.command` and
//! `crumbkit.run` have no convention, so they sit under crumbkit's own
//! namespace.

use std::process::ExitCode;

use tracing::Span;
use tracing::field::Empty;

use crate::exit;

#[cfg(test)]
mod tests;

/// Returns the span of `parse`, with the run's trace ids when the usage log
/// started.
pub(crate) fn parse() -> Span {
    // The test build installs the span tests' subscriber before any thread
    // reaches the span's callsite, so tracing never caches the callsite's
    // interest from a process without that subscriber.
    #[cfg(test)]
    tests::install();
    let span = tracing::info_span!(
        target: "crumbkit",
        "crumbkit.parse",
        "crumbkit.command" = Empty,
        "process.exit.code" = Empty,
        "error.type" = Empty,
        trace_id = Empty,
        "crumbkit.run" = Empty,
    );
    if !span.is_disabled() {
        record_ids(&span, ids());
    }
    span
}

/// Returns the span of `finish`, with the run's trace ids when the usage log
/// started.
pub(crate) fn finish() -> Span {
    // The test build installs the span tests' subscriber before any thread
    // reaches the span's callsite, so tracing never caches the callsite's
    // interest from a process without that subscriber.
    #[cfg(test)]
    tests::install();
    let span = tracing::info_span!(
        target: "crumbkit",
        "crumbkit.finish",
        "process.exit.code" = Empty,
        "error.type" = Empty,
        trace_id = Empty,
        "crumbkit.run" = Empty,
    );
    if !span.is_disabled() {
        record_ids(&span, ids());
    }
    span
}

/// Records the command path below the tool's name, such as `["books",
/// "list"]`, on `crumbkit.parse` while `parse` runs. An empty path records
/// nothing.
pub(crate) fn command(path: &[String]) {
    if let Some(span) = current()
        && !path.is_empty()
    {
        span.record("crumbkit.command", path.join(" "));
    }
}

/// Records `code`, a failure's code in the tool's spelling, as `error.type`
/// on crumbkit's span while `parse` or `finish` runs. Each run prints at
/// most one failure outside a batch, and a batch records only its first, so
/// the field is recorded once.
pub(crate) fn failed(code: &str) {
    if let Some(span) = current() {
        span.record("error.type", code);
    }
}

/// Returns the current span when it is one of crumbkit's, or `None`. When
/// the subscriber disables crumbkit's spans, the current span is the tool's
/// own, and crumbkit records nothing on it.
fn current() -> Option<Span> {
    let span = Span::current();
    let ours = span
        .metadata()
        .is_some_and(|metadata| metadata.target() == "crumbkit");
    ours.then_some(span)
}

/// Records `exit` as `process.exit.code` on `span`.
pub(crate) fn exited(span: &Span, exit: ExitCode) {
    span.record("process.exit.code", i64::from(exit::number(exit)));
}

/// Records `ids`, the trace id and the run's id as hex, on `span`. It
/// records nothing when `ids` is `None`. The callers skip it when no
/// subscriber wants the span, so a tool without a subscriber never locks the
/// recorder or formats the ids.
fn record_ids(span: &Span, ids: Option<(String, String)>) {
    if let Some((trace, run)) = ids {
        span.record("trace_id", trace);
        span.record("crumbkit.run", run);
    }
}

/// Returns the trace id and the run's id from the usage log's recorder, or
/// `None` without the `usage-log` feature or a started log.
fn ids() -> Option<(String, String)> {
    #[cfg(feature = "usage-log")]
    {
        crate::usage::ids()
    }
    #[cfg(not(feature = "usage-log"))]
    {
        None
    }
}
