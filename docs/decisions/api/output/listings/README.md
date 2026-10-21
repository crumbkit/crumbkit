# ADR-0050. Listings

- Date: 2026-10-12
- Status: amended by ADR-0077, ADR-0087, ADR-0096 and ADR-0139

## Children

- [ADR-0139. A summary that writes no text prints no line](0139-a-summary-that-writes-no-text-prints-no-line.md)

## Context

A listing's result is data that callers pipe into `wc`, `grep` or `jq`,
and the lines that describe it, such as a footer or an empty note, break
those pipes when they sit on standard output. Coding agents commonly
return standard error to the model with standard output, and a footer on
standard output can be cut away by `| head`. Listings also needed answers
for empty results, sources that cannot be counted, an upper bound on
`--limit`, and rows that report a problem.

## Decision

- In text, every line that describes a result printed on standard error,
  and standard output holds the result alone, so a pipe sees only data
  and a filtered agent still sees the notes. The summary line stays on
  standard output, because it is part of the result. crumbkit flushes
  standard output first and writes all notes in one write, so a reader
  that merges the streams sees the notes after the rows, in one piece.
  With `--json`, the document holds what the notes say.
- A listing with no rows printed its header and an empty note, whose
  wording a document can set. Its JSON is unchanged. A list of rows prints
  no note, because it has no total or description to word one from.
- `Listing::uncounted` let a source that cannot be counted cheaply, such
  as a paged remote API, report only whether more rows exist. The JSON
  then leaves out `total`, so every listing schema marks it optional, and
  the footer gives no count. `truncated` says whether any rows were left
  out, whatever the cause.
- `Page<N, MOST>` set an upper bound on `--limit` that clap enforces at
  parse time. The refusal's recovery replaces `--limit` with `--all`, so
  the caller's filters stay.
- A row type named the field that marks a problem with `Row::PROBLEM`. A
  run whose rows report a problem and that has no failure exits with 1,
  so a CI job or an agent can gate on the exit code without parsing the
  rows. `exit::problem_rows` is the one place that chooses that code, and
  the exit reads every loaded row, including rows the budget left
  unprinted.

## Consequences

`docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: notes on standard output after the result, a note on standard error only when standard output is not a terminal, the summary line on standard error, a `note` field in the JSON document, a line in a `--json` stream for the note, a note for a `Vec`, an empty note in the help of each leaf, a new usage record id for `empty`, a bound checked in the handler after parsing, a refusal code of its own for the bound, a recovery that runs `--limit MOST` first, a bound as a runtime setting of `Output`, a `--limit` that clamps a larger value to the bound without a refusal, a listing without a total that leaves out `total` on its last page too, a footer that reads `Showing 4 of at least 5 books`, `truncated` on a list, `truncated` in place of `cut`, an uncounted listing that only takes a `Vec`, a problem method on `Row` that returns a code, an exit number that a guide's `Exit` claims for problem rows, a problem exit without a note, examples that may exit with 1 for a problem.
