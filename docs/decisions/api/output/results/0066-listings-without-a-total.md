# ADR-0066. Listings without a total

- Date: 2026-10-17
- Status: superseded by ADR-0071
- Supersedes: ADR-0059, which it amends

## Context

A document with the tool's own keys kept every part of crumbkit's envelope,
so a counted listing always printed `total` (ADR-0059). Some tools already
printed listings whose readers expected no count beside the rows. Such a
tool could only keep the existing shape with `Document::without_envelope`,
which lost `truncated`, `cut` and `--full`.

## Decision

- A listing whose document declared its keys with `Document::with_keys`
  could call `Document::without_total`. Its JSON left out `total` and kept
  `truncated`, so a reader still learned whether rows were left out.
- The text footer, the output budget's cut line and the usage log still
  counted the rows, because only the JSON reader expected no count.
- The document's schema had no `total` property.
- A const panic rejected `without_total` on a document without `with_keys`,
  so crumbkit's own envelope held `total` whenever the tool counted the
  rows.
- Leaving out the count cost a reader something, so the choice was an
  opt-out (ADR-0036). crumbkit-test's `check_document_ids` failed a leaf
  whose document called `without_total` unless an opt-out from
  `documents/total` with a reason covered the command, and
  `check_opt_outs` reported what the missing count cost.

## Consequences

A tool whose readers expect a listing without a count keeps `truncated`,
`cut`, `--full` and the text footer. A program that reads such a listing
cannot learn from the JSON how many rows matched. A document without
`without_total` prints as before.

Considered: a declaration in place of an opt-out, leaving out `total` for
every document with the tool's own keys, a seam that renames `total`,
removing `total` from the text footer as well.
