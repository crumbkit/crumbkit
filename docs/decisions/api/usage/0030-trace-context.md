# ADR-0030. Trace context

- Date: 2026-10-11
- Status: accepted

## Context

An agent or a script that runs a tool may already trace its own work, and
a tool may run children. Joining a run to its caller's trace and to its
children's runs lets the usage log and any tracing tool see one chain of
work. crumbkit prefers an existing standard to a format of its own.

## Decision

- A run joined its caller's trace through W3C Trace Context, read from
  the `TRACEPARENT` and `TRACESTATE` variables of OpenTelemetry's
  environment-variable carrier. crumbkit reads only those two names,
  once, through one strict parser, so only validated hex reaches the
  record. An invalid `TRACEPARENT` starts a new trace and discards the
  caller's `TRACESTATE`, as W3C requires.
- Each run got its own run id, which its children name as their parent.
  A run that cannot read random bytes installs no recorder and gives its
  children no variables.
- The record held the trace ids unhashed, because they are random values
  that the caller already shares with its other children.
- `trace::child_env` gave a child the variables only when the usage log
  records the run, so a child never names a parent span that nothing
  records. For a new trace it sets an empty `TRACESTATE`, so a child never
  sees the state of another trace.
- crumbkit never touched `BAGGAGE`, so a child inherits it unchanged.
- The `tracing` feature, off by default, ran `parse` and `finish` in
  spans whose fields follow OpenTelemetry's conventions for CLI programs.
  crumbkit installs no subscriber, and a tool opens its own span around
  the handler, because crumbkit cannot wrap it.
- No span or record holds a value the caller typed or `TRACESTATE`.
- Per-request runs for a long-running server and OTLP export were left
  out of scope.

## Consequences

`docs/behaviour/usage-log/trace-context.md` describes the current
behaviour and the versions of the standards it follows.

Considered: a crumbkit-specific run-id variable, a free-text correlation id, always giving children this run's id, forwarding `BAGGAGE`, a crumbkit timing API, crumbkit installing a subscriber, hashing the trace ids, omitting `TRACESTATE` for a new trace, `trace.id` as the span field.
