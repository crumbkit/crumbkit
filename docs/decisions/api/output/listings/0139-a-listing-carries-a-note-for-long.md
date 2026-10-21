# ADR-0139. A listing can carry a note that prints under `--long`

- Date: 2026-10-22
- Status: accepted
- Supersedes: ADR-0138, which it amends
- Missed: ADR-0138 left out a note printed only under `--long` because it needed a new member of `Rows` while `row.rs` was near its 600-line limit, and an adopter has since asked for one.

## Context

An adopting tool has a report command whose listing gives a measured
figure for each row. Its readers need to know how the figures were
counted and where each one came from, but only when they ask for detail.
The tool's rules are that `--long` prints that explanation, and that the
default text and the JSON leave it out. `Listing::note` prints in every
text run, so the tool could not keep the explanation for `--long`. The
report's rows have no long columns, so crumbkit refused `--long` with
exit 2, and the tool could not offer the flag at all.

## Decision

- `Listing::long_note` sets a line that the text prints on standard
  error after the tool's note and before the `Next:` line, under `--long`
  only. The default text, `--fields` and the JSON document leave it out.
  `Rows` gains a hidden member that returns it, and only a listing
  returns one.
- `Document::long_noted`, a `const fn` on a listing's document, declares
  that the listing carries a long note. `parse` decides which view flags
  a leaf offers from the leaf's document before the handler builds the
  listing, so the declaration has to sit on the document. A leaf whose
  document declares a long note offers `--long` even when its row type
  has no long columns, and `finish` accepts `--long` by the same rule.
  Its `-h` and `--help` print `Long note: --long also prints a note on
  standard error.` after the `Rows:` line, because the help of `--long`
  names only the long columns.
- A row type that has long columns offers `--long` without the
  declaration, and its listing's long note prints under `--long` all the
  same. A row type that sets `Row::WHOLE` and names no columns offers no
  `--long`, as ADR-0138 decided for a document that lists only its
  problem rows.
- `--json --long` on such a listing follows ADR-0138: it is refused when
  the row type sets `Row::WHOLE`, and otherwise prints the same document
  as `--json`.
- The API-1 ground is one adopter's request for an ordinary need,
  because any tool that reports measured figures has detail about how it
  measured them that most runs do not need (ADR-0122).

## Consequences

A report command keeps its default text short and its JSON free of
prose, and a reader who types `--long` learns how the figures were
measured. A tool that sets a long note on a listing whose rows have no
long columns and forgets `Document::long_noted` gets the refusal of
`--long`, so its note never prints. `docs/behaviour/output/listings/README.md`
describes the current behaviour.

Considered: a static note on the document, which cannot name figures
known only at run time; accepting `--long` whenever the listing value
holds a long note, which `parse` cannot see before the handler runs; a
`Leaf` setting, which would split the declaration from the document that
the note belongs to; a long note on a batch and a prose result, which no
tool has asked for; printing the long note under `--fields` too; and the
names `Document::with_long_note`, `Listing::detail` and
`Listing::long_only_note`.
