# Trace context

A run of a tool joins the trace of whatever started it and passes the trace
on to its children. This page covers the standards crumbkit follows for
that, what the usage log records of the trace, and the `tracing` feature's
spans. The rustdoc of the [`trace`](../../../crates/crumbkit/src/trace.rs)
module states how a run reads the context, what
[`trace::child_env`](../../../crates/crumbkit/src/trace.rs) returns, and
which fields each span records.

## Standards and variables

A run joins its caller's trace through
[W3C Trace Context Level 1](https://www.w3.org/TR/trace-context/), and keeps
the random-trace-id flag of
[Level 2](https://www.w3.org/TR/trace-context-2/). The context travels in
the `TRACEPARENT` and `TRACESTATE` environment variables of OpenTelemetry's
[environment-variable carrier](https://github.com/open-telemetry/opentelemetry-specification/blob/v1.61.0/specification/context/env-carriers.md)
in specification v1.61.0. crumbkit reads only those two uppercase names,
reads them once at start-up, and injects the context into the environment
of a child through `trace::child_env`.

crumbkit never reads, forwards or records `BAGGAGE`, so a child inherits it
as it would without crumbkit.

Why: ADR-0030.

## Reading TRACEPARENT

crumbkit-usage's `start` reads `TRACEPARENT` through one
strict parser and ignores the `traceparent` that is-ai-agent reports, so
only hex that the parser validated reaches the record. The parser accepts a
version of two lowercase hex digits, a trace id of 32, a parent id of 16 and
flags of 2, joined by single dashes. It rejects version `ff`, an uppercase
hex digit, and a trace id or parent id that is all zero. A version `00`
value has exactly those four fields, and a higher version may add fields
after another dash, which the parser ignores.

A missing or invalid `TRACEPARENT` starts a new trace, and the run then
discards the caller's `TRACESTATE`.

Why: ADR-0030.

## Trace ids in the usage log

Each run has its own run id, which is the span id its children name as
their parent. The usage log's line records the trace id as `trace`, the
run id as `run`, and the caller's span id as `parent`, which is left out
for a new trace. The line holds the trace ids unhashed. The keyed `session`
hash groups an agent's runs, as the
[usage log](README.md#caller-and-session) page describes.

A run that cannot read random bytes installs no recorder, so it writes no
line and gives its children no variables. `trace::child_env` returns
variables only when the usage log is a
[run observer](../run-observer/README.md) and the run writes a line, or
another observer gives them, because it asks the observers for them, so a child never names a parent span that
nothing records.

Why: ADR-0030, ADR-0057.

## Passing the trace to a child

`trace::child_env` passes on only the flag bits that Level 1 and Level 2
define. Every child of one run gets the same variables, so children that run
one after another or in parallel all name the run as their parent.

When the run started a new trace and the caller set `TRACESTATE`,
`child_env` returns an empty `TRACESTATE`. The empty value replaces the
caller's value in the child's environment, so a child never sees the state
of another trace. W3C Trace Context Level 1 section 3.3.1.1 requires
vendors to accept an empty tracestate, and OpenTelemetry's propagator
requirements treat an empty trace state as no state.

Why: ADR-0030.

## Spans of the tracing feature

The `tracing` cargo feature, off by default, runs `parse` and `finish`
inside spans of the `tracing` crate. The spans record the trace id and run
id of the first run observer that gives them. crumbkit installs no subscriber. A tool
that wants one span per run opens its own span in `main`, and crumbkit's
spans nest inside it.

The spans' fields follow OpenTelemetry's
[semantic conventions for CLI programs](https://github.com/open-telemetry/semantic-conventions/blob/v1.44.0/docs/cli/cli-spans.md),
v1.44.0 with Development status, where a convention exists. The trace id
field is `trace_id`, which is the name OpenTelemetry gives the trace id in
log formats other than OTLP.

Nothing that crumbkit records holds a value the caller typed. No span holds
an argument's value, the command line or `TRACESTATE`. shelf's end-to-end
test plants a secret in `TRACEPARENT` and `TRACESTATE` and fails when the
usage log holds it.

Why: ADR-0030, ADR-0057.

## Scope

crumbkit does not provide per-request runs for a long-running server, and it
does not export spans over OTLP.

Why: ADR-0030.
