//! The run's W3C trace context: the trace it joins, its own id, and the
//! variables that put its children in the same trace (ADR-0030).

use std::ffi::OsString;

use super::platform;
use super::record::{SpanId, TraceId, TraceIds};

#[cfg(test)]
mod tests;
mod traceparent;

/// The trace flags that a run keeps from its caller and passes on: sampled
/// (0x01) and random trace id (0x02). W3C Trace Context Level 1 requires the
/// other bits to be zero when they go on.
const KNOWN_FLAGS: u8 = 0x03;

/// The run's trace context.
#[derive(Debug)]
pub(super) struct Context {
    /// The trace id, the run's id and the caller's span id.
    pub(super) ids: TraceIds,
    /// The trace flags that the run's children receive.
    flags: u8,
    /// The TRACESTATE that the run's children receive: the caller's with a
    /// valid TRACEPARENT, an empty value for a new trace when the caller set
    /// one, and none when the caller set none.
    state: Option<String>,
}

/// Returns the context of this run from its TRACEPARENT and TRACESTATE and
/// 24 random bytes from the operating system, or `None` when the bytes cannot be
/// read or an id would be all zero.
pub(super) fn read() -> Option<Context> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, which give the caller's trace context"
    )]
    let env = |name: &str| std::env::var(name).ok();
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, which give the caller's trace context"
    )]
    let state = std::env::var_os("TRACESTATE");
    let bytes: [u8; 24] = platform::random_bytes()?;
    let (trace, run) = bytes.split_first_chunk::<16>()?;
    Context::new(
        env("TRACEPARENT").as_deref(),
        tracestate(state),
        *trace,
        run.try_into().ok()?,
    )
}

/// Reports whether the run answers a shell's completion request, which
/// writes no line.
pub(super) fn completing() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, and a completion request writes no line"
    )]
    completing_in(|name| std::env::var_os(name))
}

/// Reports whether the run answers a shell's completion request, with `env`
/// giving the value of an environment variable by its name. It reads the
/// completion variable that the tool declared with `crumbkit::complete_with`,
/// or `COMPLETE` (ADR-0048).
fn completing_in(env: impl FnOnce(&str) -> Option<OsString>) -> bool {
    let value = env(crate::complete::variable());
    crate::complete::requested(value.as_deref())
}

/// Returns the caller's TRACESTATE from `value`, the variable as the
/// environment holds it. A value that is not UTF-8 is not a valid
/// TRACESTATE, and it counts as empty, so the run still knows that the
/// caller set the variable.
fn tracestate(value: Option<OsString>) -> Option<String> {
    value.map(|state| state.into_string().unwrap_or_default())
}

impl Context {
    /// Returns the context of a run whose caller set `traceparent` and
    /// `tracestate`, with `run` as the run's id.
    ///
    /// A valid `traceparent` gives the caller's trace id, its span id as the
    /// parent, and its flags. Without one, the run starts a new trace whose
    /// id is `new_trace`, with no parent and the flags `03`, and discards the
    /// caller's TRACESTATE, as W3C requires. When the caller set TRACESTATE,
    /// the new trace keeps an empty one, which the run gives its children in
    /// place of the caller's. It returns `None` when the id that the run would use
    /// from `new_trace` or `run` is all zero, which W3C forbids.
    pub(super) fn new(
        traceparent: Option<&str>,
        tracestate: Option<String>,
        new_trace: [u8; 16],
        run: [u8; 8],
    ) -> Option<Self> {
        if run == [0; 8] {
            return None;
        }
        let run = SpanId::new(run);
        if let Some(parent) = traceparent.and_then(traceparent::parse) {
            return Some(Self {
                ids: TraceIds {
                    trace: TraceId::new(parent.trace),
                    run,
                    parent: Some(SpanId::new(parent.parent)),
                },
                flags: parent.flags & KNOWN_FLAGS,
                state: tracestate,
            });
        }
        if new_trace == [0; 16] {
            return None;
        }
        Some(Self {
            ids: TraceIds {
                trace: TraceId::new(new_trace),
                run,
                parent: None,
            },
            flags: KNOWN_FLAGS,
            state: tracestate.map(|_| String::new()),
        })
    }

    /// Returns the environment variables that put a child process in this
    /// run's trace with this run as its parent: TRACEPARENT in version `00`,
    /// and TRACESTATE when the caller set one. TRACESTATE is the caller's,
    /// unchanged, in the caller's trace, and empty in a new trace, so the
    /// child never inherits a TRACESTATE that belongs to another trace.
    pub(super) fn child_env(&self) -> Vec<(&'static str, String)> {
        let mut env = vec![(
            "TRACEPARENT",
            format!(
                "00-{}-{}-{:02x}",
                self.ids.trace.as_str(),
                self.ids.run.as_str(),
                self.flags
            ),
        )];
        if let Some(state) = &self.state {
            env.push(("TRACESTATE", state.clone()));
        }
        env
    }
}
