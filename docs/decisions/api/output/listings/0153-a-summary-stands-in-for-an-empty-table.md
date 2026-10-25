# ADR-0153. A listing with a summary and no rows prints the summary alone

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0050 and ADR-0138, which it amends
- Missed: ADR-0138 printed the summary line alone only for a listing that lists only its problem rows, although any listing with a summary and no rows to print has the same header with nothing under it.

## Context

ADR-0050 prints the header of a listing with no rows, followed by the
empty note on standard error. ADR-0138 prints the summary line alone,
with no header, when a listing that lists only its problem rows has none
to print. An adopting tool lists the services of a group under a summary
line that names the group and its state. When the group has not been
started, the listing has no rows, and the text printed the summary, then
a header with nothing under it. The header tells the reader nothing that
the summary and the empty note do not say, and the tool's earlier
renderer printed the summary line alone.

## Decision

- A listing that has a summary line and holds no rows to print prints the
  summary line and no header. The rule of ADR-0138 becomes one case of
  this rule: the text prints a header only when the listing holds a row to
  print, or when no summary line stands in its place. A budget cut that
  leaves no row printed does not drop the header, because the listing
  still holds rows.
- The rule holds for every text view of the listing, the default table,
  `--long` and `--fields`, because none of them has a row to print under
  the header.
- A listing without a summary still prints its header, as ADR-0050
  decided. A summary whose `Display` writes no text prints no line
  (ADR-0142), so nothing stands in place of the header, and the listing
  prints its header as a listing without a summary does. Standard output
  of an empty listing therefore always holds a line.
- The footer, the empty note, the problem note, the tool's note, the
  `Next:` line and the exit do not change. `--json` does not change, and a
  stream and a batch have no summary.
- crumbkit adds no public API, so API-1 asks for no ground. The change
  serves an ordinary need, because any listing with a summary can come up
  empty (ADR-0122).

## Consequences

An empty listing with a summary prints one line on standard output. A
reader that took the first line after the summary as the header of an
empty listing finds no header and must read the empty note or `--json`.
`docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: a `Document` option that drops the header, which would let
two listings with summaries print an empty result differently for no
reason a reader can see; dropping the header of every empty listing,
summary or not, which would leave a listing without a summary printing
nothing on standard output; and keeping the header under `--fields`,
whose caller named the columns, although no row follows it.
