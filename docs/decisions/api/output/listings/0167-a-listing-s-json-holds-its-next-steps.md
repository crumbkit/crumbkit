# ADR-0167. A listing's JSON document holds its next steps under `next`

- Date: 2026-11-01
- Status: accepted
- Supersedes: ADR-0050 and ADR-0155, which it amends

## Context

A listing names its next steps with `Listing::next` and `Listing::step`.
The text prints them on standard error, and ADR-0050 and ADR-0155 left them
out of the JSON document, because no program had asked to read them. A
program that reads a tool's JSON now asks for them, such as a local web page
that draws a tool's commands and turns each next step into a button. Without
them in the document, such a program has to run the command a second time
for its text and parse the `Next:` lines.

## Decision

- A listing's JSON document holds an optional `next`: an array of steps in
  the order the text prints them. Each step is an object with `text`, a
  string, and `commands`, an array in which each command is either
  `{"call": ...}`, a command line of the tool, or `{"run": ...}`, a command
  line of another program.
- `Listing::next` is the first step, with its phrase as `text` and its call
  as its one command. Each `Listing::step` follows, with its sentence as
  `text` and its commands in order. A step without commands holds
  `"commands": []`.
- A `call` string is the command line that the text prints for that call,
  rendered against the tool's command tree. A `run` string is the line that
  the text prints for the other program, with its POSIX quoting. `finish`
  renders each step once and writes the text or the JSON from the same
  rendering.
- The JSON leaves out a step by the rule that leaves it out of the text: a
  call that does not resolve, or an argument of another program that holds
  a control character or a Unicode format character. The text and the JSON
  therefore name the same steps. A document whose listing has no step left
  holds no `next` key.
- The key order is `schema`, `dry_run`, `data`, `total`, `truncated`,
  `summary`, `next`, `cut` and `failures`.
- A listing that the output budget cuts keeps its steps in the text, so its
  JSON document keeps the same `next`, and the budget counts its bytes
  (ADR-0046). A document without the envelope prints the rows alone and has
  no `next`.
- The listing envelope schema gains the optional `next` property with an
  item schema. Every listing schema keeps its `v1` id, because a new
  optional field keeps the id (ADR-0007).

## Consequences

A program that reads `--json` sees the same next steps that a person reads
in the text, as data it can act on. Every checked-in listing schema grows by
the `next` property, and the JSON of every listing that names a step grows
by those steps. shelf's `schemas show shelf/books-list/v1` example grows by
about 840 bytes, so shelf's path budget rises from 4,300 to 5,300 bytes.
`docs/behaviour/output/listings/README.md` describes the current behaviour.

Considered: a notes line on standard error behind an environment variable,
which a program would have to parse and which the JSON would still lack; a
single object for `Listing::next` alone, which leaves out the steps that
`Listing::step` adds; a `kind` tag on each step, which a reader does not
need, because each command's key already says whether it is a `call` or a
`run`; and leaving the JSON unchanged and having readers run the command a
second time for the text, which costs a run and parses lines that were
written for people.
