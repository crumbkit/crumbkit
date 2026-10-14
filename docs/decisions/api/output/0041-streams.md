# ADR-0041. Streams

- Date: 2026-10-12
- Status: accepted

## Context

Some commands print rows as they arrive, such as a watch or a command
that reads a large file. `finish` prints a whole result at the end, so
such a command needed a form that prints each row at once while keeping
crumbkit's flags, failures, exit codes and budget.

## Decision

- A command declared its document as `Document<Stream<R>>`, so the
  document constant decides the command's form everywhere and the
  compiler refuses a stream document passed to `finish`.
- `stream` returned a `Streaming` whose `item` returns `ControlFlow`, so
  the tool stops producing rows when the reader closes the pipe. Opening
  the stream checks the view flags before anything prints.
- In text, each column is as wide as its header, because the stream
  cannot know the widths of rows still to come.
- With `--json`, each row printed as a whole envelope document on its own
  line, so the output is JSON Lines and a reader can process each line
  alone.
- The stream flushed after every line, so a reader sees each row at once.
- A failure in the middle printed between the rows with `--json`, or on
  standard error in text, and the stream goes on. A stream exits with its
  first failure's code, as `finish` does.
- Help, `commands list` and `schemas list` named the form, with new
  optional fields that keep the `v1` ids.
- `check_examples` stops a run after 10 seconds, so a stream example
  reads a finite input, and checks that each `--json` line is an envelope
  of the leaf's document.
- A stream printed at most the output budget, with its first item always
  printed.

## Consequences

`docs/behaviour/output/streams.md` describes the current behaviour.

Considered: a stream as a fourth `Shape` of `Rows`, a separate `Streaming` type for each format, a header measured from the first rows, no text header, a stream's notes on standard output in text, one JSON array printed as rows arrive, a JSON document per line without the envelope, an `items` count at the end of a stream, a buffered stream that flushes on a timer, failures of a stream always on standard error, a closure that `stream` calls in place of a returned `Streaming`, a `form` column in the text of `commands list`, a new id for `schemas list` and `commands list`, no timeout in `check_examples`, a follow mode in the reading example.
