# ADR-0188. A run takes a file as input, copies output and inherits output

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0171 and ADR-0172, which it amends
- Missed: ADR-0171 limited a child's input to null, bytes or inherited and its output to four policies, and ADR-0172's time bound did not consider a caller's function that runs on a reader thread.

## Context

A tool that runs other programs often gives a child a file or a pipe as its
input, shows a child's output to a person while it keeps a bounded copy, or
leaves a stream on the tool's own for an interactive step. ADR-0171 allowed
none of these, so such a tool cannot use `run` for those children.

## Decision

- `Input::File` takes an `Arc<File>`, which keeps `RunOptions` cloneable. The
  child gets a duplicate that shares the file's position, and no writer
  thread starts.
- `Capture::Inherit` leaves a stream on the tool's own. The run starts no
  reader for it, so a copy function of an inherited stream receives nothing.
- `RunOptions::stdout_copy` and `stderr_copy` give a function each chunk of a
  stream as the run reads it, beside any capture, which keeps its limit.
- A copy function runs on the reader thread. The time that it blocks is
  outside the time bound of a run that ADR-0172 states, because a scoped
  thread must be joined before the run returns.
- A copy function that panics gets no more chunks. The reader goes on reading
  so that the child runs on, and the run ends with `RunError::Run`.
- `LongLine::Skip` is a third result for a line over its limit. The run drops
  the line and goes on, where ADR-0171 either stops the child or sinks the
  rest.
- `Input::Bytes` holds a `Zeroizing<Vec<u8>>`, and the crate re-exports
  `Zeroizing`, so the crate's public API follows zeroize's major version.

## Consequences

A tool can feed a child from a file or a pipe, show its output, and hold an
interactive step without leaving `run`. A copy function that blocks holds the
child and the run, and the function's author owns that wait.

Considered: an owned descriptor in `Input`, which would make `RunOptions`
single-use; a writer inside `Capture`, which would need one form for each
capture policy; a time limit on a copy function, which cannot end a thread
that must be joined.
