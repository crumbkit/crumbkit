# ADR-0082. View flags on a failed run

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0016, which it amends

## Context

Under ADR-0016, `parse` refused a view flag that the leaf's document could
not use, and `finish` judged the view flags again against the document that
the tool passed to it, before it looked at the handler's result. A tool may
finish an early failure, such as a missing config file, in another document
than the one its leaf declares, for example a prose document. `finish` then
refused a `--long` or `--fields` that the leaf's listing offers, as
`<tool>::cli::usage` with exit code 2, and the failure that the caller
needed to read never printed.

An unknown `--fields` name was refused only by `finish`, against the row
type of the document it was given. On a failed run, whether a name was
refused therefore depended on which document the tool passed with the
failure.

## Decision

- `parse` refuses a `--fields` name that the row type of the leaf's
  document lacks, before the handler runs, with the same code, message and
  hint that `finish` gives an unknown name in a result. A leaf without a
  document has no row type to check against.
- When the handler returns a failure, `finish` refuses only two view flags
  that cannot work together, which does not depend on any document, and
  otherwise prints the failure with its own code.
- When the handler returns a result, `finish` judges the view flags against
  the document it was given, as before.

## Consequences

A failed run on a leaf with a document prints its failure for every view
flag that the leaf's document accepts, and a mistyped flag or name fails
before the handler runs. A tool that parses with clap alone, a tool that
builds its own `ViewFlags`, and a leaf without a document get no check from
`parse`, so their failed runs print the failure even for a mistyped
`--fields` name. The rest of ADR-0016 is unchanged.

Considered: printing the failure before every view-flag check, including an
unknown `--fields` name; refusing an unknown name on a failed run only when
the document passed to `finish` has rows; judging a failed run's flags
against the leaf's declared document inside `finish`.
