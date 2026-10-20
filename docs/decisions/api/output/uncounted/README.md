# ADR-0087. Uncounted listings omit totals

- Date: 2026-10-18
- Status: amended by ADR-0091
- Supersedes: ADR-0050, which it amends
- Missed: ADR-0050 gave the last page of an uncounted listing the number of rows it held as its total, and did not check that total against the output budget, which cuts rows after the tool has handed them over.

## Children

- [ADR-0130. A capped source gives a lower bound](0130-a-capped-source-gives-a-lower-bound.md)

## Context

`Listing::uncounted(rows, more)` declared that the tool did not count the
rows of its source. While `more` was true, the listing had no total. On the
last page, where `more` was false, the listing was the counted listing that
`Listing::new` returns, with the number of rows it held as its total.
`Page::uncounted` with `--all`, and with a source that ended within the
page, built that last page.

When the output budget cut such a page, its JSON document held `"total": 5`
beside `"truncated": true` and `cut`, and its text cut line read `Showing 1
of 5 entries, because output stops at 60 bytes.` When the tool cut the
rows itself, by leaving `more` true, neither form gave a count. The same
declaration therefore printed a total on one path and none on the other,
and a reader of the JSON could not rely on the absence of `total` to tell
an uncounted listing from a counted one.

## Decision

- A listing from `Listing::uncounted` or `Page::uncounted` has no total,
  whether or not more rows exist. Its JSON document leaves out `total`, and
  its footer and its cut line give no count, whatever left rows out: the
  output budget, the page's limit or the tool.
- The listing keeps `more`, so `truncated`, the footer and the usage log
  still say whether rows were left out. A listing that holds every row of
  its source has `"truncated": false` and no footer, unless the output
  budget cut it, when it has `"truncated": true` and a cut line.
- An uncounted listing with no rows and no more rows prints the empty note,
  as a counted listing with a total of 0 does. Its JSON leaves out `total`.
- A counted listing prints as before.

## Consequences

A reader learns that a tool counted its rows from the presence of `total`.
A tool that knows the length of its source, such as a `Vec` it holds
whole, calls `Listing::new` or `Listing::from_all` to print a total.
`docs/behaviour/output/listings/README.md` describes the current behaviour.

Considered: keeping the total on the last page and dropping it only when
the budget cut the page, a `total` of `null`, a `counted` field in the JSON
document.
