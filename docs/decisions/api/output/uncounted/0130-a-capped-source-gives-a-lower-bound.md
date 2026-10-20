# ADR-0130. A capped source gives a lower bound

- Date: 2026-10-21
- Status: accepted

## Context

An adopting tool lists records from a store that has no bound on its size.
For speed, the tool reads only the newest 100 records, and its read reports
whether it left older records unread. The tool then shows a page of those
100 records with `Listing::new(rows, 100)` or `Listing::from_all(read,
limit)`.

The footer of that page read `Showing 20 of 100 records. --all shows every
one.` The sentence was false in two ways: the store holds older records
beyond the 100, and `--all` shows only the 100 records that the tool read.
The JSON document held `"total": 100`, which by ADR-0087 means that the tool
counted every matching row.

The change meets API-1 on the third ground of ADR-0122: one adopter asked
for it, and the need is ordinary, because any tool that reads a bounded
window of a large store, such as the newest entries of a history or the
first pages of a remote API, has the same footer and the same `total`.

## Decision

- `Listing::capped(self, capped: bool) -> Self` says whether the tool read
  only part of its source. With `capped` true, the listing's total counts
  the rows that the tool read, and the source holds more rows beyond them.
  With `false` it changes nothing, so a tool writes `Listing::new(rows,
  read).capped(history_was_cut)` in one expression.
- `capped` applies to a listing from `Listing::new`, `Listing::from_all`,
  `Page::listing` and `Page::first_of`. A listing from `Listing::uncounted`
  has no total, so `capped` changes nothing on it.
- A capped listing has no exact total, so by ADR-0087 its JSON document
  leaves out `total` and holds `"truncated": true`. The JSON document gains
  no field.
- While the page shows fewer rows than the tool read, the footer gives the
  rows read as a lower bound and says how many rows `--all` shows:
  `Showing 20 of at least 100 records. --all shows 100 of them.` A bound on
  `--limit` adds its clause, as in `Showing 20 of at least 100 records.
  --limit takes up to 100, and --all shows 100 of them.` A listing without
  a description reads `Showing 20 of at least 100. --all shows 100 of
  them.`
- When the page shows every row that the tool read, as with `--all`, the
  footer reads `Showing 100 records, and more match.` and names no `--all`,
  because `--all` shows nothing more. One row takes the singular, as the
  footer of an uncounted listing does, and a listing without a description
  says `rows` or `row`.
- The output budget's cut line of a capped listing gives no count, as an
  uncounted listing's does.
- The usage log marks the run as one whose listing left rows out.
- The tool's note, summary and `Next:` line print as before.

## Consequences

A reader of the JSON document cannot tell a capped listing from an
uncounted one, and both lack `total` because neither has an exact count. A
person reading the text footer learns the lower bound and what `--all`
shows. A tool that wants the reader of the JSON document to know the number
of rows it read adds that number to the listing's summary.
`docs/behaviour/output/listings/README.md` describes the current behaviour.

Considered: keeping `total` in the JSON document beside a new `capped`
field, which leaves a `total` that is not an exact count; a lower-bound
field such as `total_at_least`, which adds a field that every reader must
learn; a note line only, which leaves the false `--all` sentence in place;
`Listing::uncounted`, which loses the lower bound and still says `--all
shows every one`.
