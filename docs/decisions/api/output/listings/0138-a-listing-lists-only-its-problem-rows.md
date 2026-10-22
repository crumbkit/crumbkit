# ADR-0138. A listing can list only its problem rows

- Date: 2026-10-21
- Status: amended by ADR-0139 and ADR-0140
- Supersedes: ADR-0065 and ADR-0120, which it amends
- Missed: ADR-0065 refused `--full` for a row type that sets `Row::WHOLE` because its JSON already holds every field, and it left `--long` with `--json` accepted although `--long` adds nothing to that JSON either. ADR-0120 gave a row one set of child lines, so a tool could not keep its warnings for `--long`.

## Context

An adopting tool has a check command that returns a listing of whole
rows, one for each item it checks. Each row overrides `Row::is_problem`
and carries its errors and warnings as child lines. The tool's rules are
that the default text lists only the failing rows with their errors,
under the summary line, that `--long` lists every row and every warning,
and that `--json` with `--long` is refused. crumbkit could print neither
form. A handler could not filter the rows itself either, because `Output`
reports whether the caller gave `--json` and not whether the caller gave
`--long`, and a handler that filtered its rows would also change the JSON.

## Decision

- `Document::only_problems`, a `const fn` on a listing's document, makes
  the default text list only the rows that report a problem. The text
  without `--json`, `--long` or `--fields` prints the summary line and the
  table of those rows with their child lines, and prints the summary line
  alone when no row reports one. `--long` and `--fields` print every row,
  and `--json` holds every row unchanged. The footer, the empty note of a
  listing without rows, the problem note and the exit do not change, and
  no line counts the rows that the text leaves out.
- The option sits on the document, because the same row type can be
  listed by a command that lists every row. It is limited to a listing,
  the shape of a check command's result.
- A leaf whose document sets the option offers `--long` even when its row
  type has no long columns, because `--long` then lists every row. Its
  `-h` and `--help` print a `Rows:` line that says what the default lists
  and that `--long` or `--json` lists every row.
- `Row::long_child_lines` returns the child lines that the text prints
  under `--long`, and by default returns `Row::child_lines`, so every
  existing row type prints the same bytes. The table, a stream and the
  measure of a batch's output budget use it under `--long`. JSON does not
  change.
- crumbkit refuses `--long` with `--json` for a row type that sets
  `Row::WHOLE`, by the reasoning that refuses `--full` for it: its JSON
  already holds every field, so `--long` adds nothing. `parse` refuses it
  before the handler runs, and `finish` and `stream` refuse it by their
  document, as a refused command line with the code `<tool>::cli::usage`,
  exit 2 and the hint to drop `--long`. Before this record, the same
  command line printed the same document as `--json` alone, so the
  refusal removes a spelling that added nothing.
- A listing that lists only its problem rows and whose row type neither
  sets `Row::WHOLE` nor has long columns accepts `--json --long`, which
  prints the same document as `--json`. crumbkit accepts it, because the
  refusal is a property of the row type, and this case depends on the
  document as well.
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool that checks many items needs its default text to show the
  failures without the passing items around them (ADR-0122).

## Consequences

A check command keeps its default text short and its JSON whole, and a
reader reaches every row through `--long`. A script that typed `--json
--long` on a command whose rows print whole now fails with exit 2 and
must drop `--long`. When a limit cuts the listing or the output budget
cuts the text, the footer and the cut line count the rows of the listing
or of the printed table, without saying that the text left out rows that
report no problem. `docs/behaviour/output/listings/README.md` describes
the current behaviour.

Considered: `Output::is_long` with the filter in each handler, which
would also filter the JSON unless every handler checked `--json` too; a
`Row` constant in place of a document option; a footer that counts the
rows left out; an empty note when no row reports a problem; an explicit
guide setting to refuse `--long` with `--json`; `Listing::long_note`, a
note printed only under `--long`, which needs a new member of `Rows` and
waits for a second use; and the names `problems_only`, `failures_only`
and `hide_healthy`.
