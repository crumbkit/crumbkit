# ADR-0144. A tool adds value-free facts of its own to the usage record

- Date: 2026-10-23
- Status: accepted
- Supersedes: ADR-0095, which it amends
- Missed: ADR-0095 removed `section` because no tool called it and crumbkit could not check what it held, and it did not consider a section whose types keep values out, which a tool that calls a network API needs.

## Context

An adopting tool that calls an HTTP API wants its usage log to record, for
each run, facts that crumbkit cannot see: how many requests it sent, their
status classes, the route templates it called, the time it waited for the
server, the bytes and retries, and which kind of target it addressed.
Without them, `self usage` cannot tell a slow command that waits on a
server from one that is slow on its own. ADR-0095 removed `section`, which
took any serializable value, because no tool called it and crumbkit could
not keep a value out of it.

## Decision

- A tool keeps its facts in a type of its own that implements the
  `Extension` trait, and changes them during the run with
  `crumbkit_usage::extend`, which creates the value with `Default` on first
  use and does nothing without a started log. When the line is written,
  each extension writes its facts through `Facts`, which a tool cannot
  build.
- `Facts` takes three kinds of fact: a count under a `Key`, a `Label` under
  a `Key`, and a time spent waiting on another system. A `Key` and a
  `Label` hold a `&'static str` that a `const fn` checks against a small
  alphabet, so a tool declares them as constants, a bad one fails to
  compile, and a value read at run time can become a label only by leaking
  it on purpose.
- The line holds the facts under one `facts` field whose shape crumbkit
  fixes: `counts`, `labels` and `waited_ms`. The `<tool>/usage/v1` schema
  describes it, so the record keeps its `v1` id. A run may keep several
  extensions, and their facts share the section: counts under one key add,
  and labels under one key are kept once each.
- A line that would pass 1 KiB with its facts is written without them, so
  the facts never cost a run its line.
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool that calls another system wants its usage log to record those
  calls (ADR-0122). shelf's `books show` records its lookups as a use in
  `examples/`.

## Consequences

The record again has a field that the tool fills, and crumbkit's types
limit it to numbers and the tool's own declared vocabulary. A tool can
still put a value in a count, such as a book id, which review has to
catch. The report reads only `waited_ms` from the section (ADR-0145), so a
hand-edited line cannot put text in the report through it.
`docs/behaviour/usage-log/README.md` describes the current record.

Considered: restoring `section` with any `Serialize` value, which crumbkit
cannot check; a schema of the tool's own for the section, which `self usage`
could not read; a derive macro that checks each field of the tool's type,
which costs a proc-macro for three kinds of fact; label text as `&str`
checked at run time, which a typed value passes; one extension per run;
dropping a line that its facts make too long; durations under any key,
which no finding reads yet.
