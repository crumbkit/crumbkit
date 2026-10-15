# ADR-0063. Rows without columns

- Date: 2026-10-15
- Status: accepted

## Context

`Row::COLUMNS` could be empty. A listing of such a row type printed `[]`
under `data` with `--json` and without `--full`, and an empty table in
text, while `total` and `truncated: false` still said that every row was
printed. The rows disappeared without a failure, a note or a nonzero exit.
The row type of a prose or bytes result, `NoRow`, has no columns and no
values, and a headed batch prints each row's bytes in text and every field
in JSON, so neither loses rows when its row type names no column.

## Decision

- `finish` and `finish_to` failed to compile for a single row, a `Vec` or
  a `Listing` whose row type has an empty `Row::COLUMNS`, and `stream` and
  `stream_to` failed to compile for such a row type. Each check was an
  inline `const` block in a function that those functions call, so the
  compiler evaluates it once for each result type and the run does no work
  for it.
- A batch passed the compile-time check, because `Document::headed` is a
  value of the document and the compiler cannot tell a headed batch from
  another batch through the result type. crumbkit-test's `check_guide`
  failed a leaf whose document holds rows, is not headed, and whose row
  type has no compact column.
- A headed leaf counted as having columns in help, so its `-h` pointed at
  the JSON fields that `--help` lists also when its row type named none.

## Consequences

A tool finds an empty `COLUMNS` on a row, list, listing or stream when it
builds, and on a batch that is not headed when its guide test runs. A
batch whose guide has no leaf for its command is not checked.
`docs/behaviour/output/README.md` states the rule.

Considered: a runtime refusal in `finish`, printing every field when the
row type has no columns, a minimum length on `COLUMNS` through a separate
trait, a check in `Document::new` that needs a bound on every document's
result type, a compile-time check on every batch that would refuse a
headed batch without columns.
