# ADR-0244. A failure lists its causes

- Date: 2026-11-17
- Status: accepted
- Supersedes: ADR-0001 and ADR-0007, which it amends

## Context

A failure printed one `cause:` line for each error in its `source()` chain,
and the JSON document joined those errors with `: ` into one `cause` string,
which says that each error caused the one before it. A tool whose failure has
several side-by-side reasons, such as one finding for each invalid item, got
one line for each reason only by building a fake linked chain of error
sources. One adopter keeps a module of 64 lines that does exactly that, and
its JSON then claims that each reason caused the one before. ADR-0007
considered `cause` as an array and rejected it, because a chain read as one
line is what a program shows.

## Decision

- `Failure::causes` returns `Vec<String>`, the reasons the failure lists side
  by side, in order, one sentence each. The default is an empty list, which
  does not allocate.
- The text form prints the `cause:` lines of the `source()` chain first and
  then one `cause:` line for each listed cause. With the `miette` feature the
  `at:` lines follow the causes. Each line escapes control and format
  characters as every other line does.
- The failure document and each entry of a batch's `failures` hold an
  optional array of strings `causes`, after `cause` and before `fields`,
  present only when the failure lists at least one cause. `cause` keeps the
  joined source chain. The id stays `v1`, because ADR-0007 keeps the id when
  an optional field is added.
- A reason with its own labels stays a miette `related` diagnostic, and a
  reason that is a value stays a field. `ReportFailure` lists no causes.
- `#[derive(Failure)]` takes `causes = EXPR`, a `Vec<String>` that reads the
  variant's fields. A forwarding variant forwards `causes()`, and `causes`
  beside `forward` is a compile error. The derive writes `causes()` only when
  a variant gives the key or forwards.

## Consequences

A failure that lists no causes prints the same bytes as before, so no golden
file changes. The published failure schema gains the optional property
`causes`, and each schema file that embeds it is regenerated. A tool that
built a fake chain of sources can list its reasons instead. A JSON reader
that rejects unknown properties refuses a document with `causes`, which
ADR-0007 already forbids.

Considered: retyping `cause` as an array, which needs a `v2` id, a structured
cause with its own code and fields, a limit on the number of causes, printing
miette's related diagnostics in text.
