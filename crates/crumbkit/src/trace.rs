//! W3C Trace Context: a run joins the trace of whatever started it, and
//! passes the trace on to its children (ADR-0030).
//!
//! A caller, such as a CI agent, a shell with OpenTelemetry, a coding agent
//! with tracing on or another crumbkit tool, puts its trace context in the
//! `TRACEPARENT` and `TRACESTATE` environment variables, as OpenTelemetry's
//! environment-variable carrier describes. With the `usage-log` feature,
//! `usage::start` reads both once. A valid
//! `TRACEPARENT` gives the run the caller's trace id and names the caller's
//! span as the run's parent. A missing or invalid one starts a new trace with
//! a random id and no parent, and the run then discards the caller's
//! `TRACESTATE`. Each run also has its own random id, and the usage log's
//! line records the three ids as `trace`, `run` and `parent`.
//!
//! A tool passes the trace to a child process with [`child_env`]:
//!
//! ```no_run
//! let status = std::process::Command::new("git")
//!     .arg("status")
//!     .envs(crumbkit::trace::child_env())
//!     .status();
//! ```
//!
//! Every child of one run gets the same variables, so children that run one
//! after another or in parallel all name the run as their one parent.
//! crumbkit never reads, forwards or records `BAGGAGE`, so a child inherits
//! it from the environment as it would without crumbkit.
//!
//! With the `tracing` feature, `parse` and `finish` each run inside a span
//! of the `tracing` crate, named `crumbkit.parse` and `crumbkit.finish`, at
//! level `INFO` with the target `crumbkit`. The spans reach whatever
//! subscriber the tool installs, and crumbkit installs none.
//!
//! - `crumbkit.parse` records `crumbkit.command`, the command path below the
//!   tool's name, such as `books list`. When `parse` ends the run with help,
//!   the version or a refusal, it also records `process.exit.code` and, for
//!   a refusal, `error.type`.
//! - `crumbkit.finish` records `process.exit.code`, and `error.type` when the
//!   result is a failure or a batch with a failure. `error.type` is the
//!   failure's code in the tool's spelling, such as
//!   `shelf::books::not_found`, and a batch gives its first failure's code.
//! - With the `usage-log` feature and a started log, both spans also record
//!   `trace_id`, the run's trace id, and `crumbkit.run`, the run's own id.
//!
//! No span holds an argument's value, the command line or `TRACESTATE`.
//!
//! crumbkit cannot wrap the tool's handler, which runs between `parse` and
//! `finish`. A tool that wants one span for the whole run opens its own span
//! in `main`, and crumbkit's spans nest inside it:
//!
//! ```no_run
//! use std::process::ExitCode;
//!
//! # fn run() -> ExitCode { ExitCode::SUCCESS }
//! fn main() -> ExitCode {
//!     // Install the tool's subscriber here.
//!     let span = tracing::info_span!("shelf", "process.executable.name" = "shelf");
//!     let _entered = span.enter();
//!     run()
//! }
//! ```

#[cfg(feature = "tracing")]
pub(crate) mod spans;

/// Returns the environment variables that put a child process in this
/// run's trace, with this run as the child's parent, for
/// `std::process::Command::envs`.
///
/// When this run is recorded, which means the usage log started and the run
/// reached `parse`, it returns `TRACEPARENT` as
/// `00-<trace id>-<run id>-<flags>`. The flags keep the caller's sampled
/// bit (`01`) and random-trace-id bit (`02`) and clear the others, and a new
/// trace has the flags `03`. When the caller set `TRACESTATE`, it also
/// returns `TRACESTATE`: the caller's value unchanged when the caller's
/// `TRACEPARENT` was valid, and an empty value when the run started a new
/// trace. An empty `TRACESTATE` holds no entries, so a child started with
/// `Command::envs` never inherits the state of the caller's trace beside the
/// new trace's `TRACEPARENT`. When the caller set no `TRACESTATE`, it returns
/// no `TRACESTATE`.
///
/// When nothing records this run, it returns no variables, so a child
/// inherits the caller's `TRACEPARENT` and `TRACESTATE` unchanged, and never
/// names a parent that nothing records. It exists in every build, so a tool
/// calls it without a feature of its own, and without the `usage-log` feature
/// it always returns no variables.
#[must_use]
pub fn child_env() -> Vec<(&'static str, String)> {
    #[cfg(feature = "usage-log")]
    {
        crate::usage::child_env()
    }
    #[cfg(not(feature = "usage-log"))]
    {
        Vec::new()
    }
}
