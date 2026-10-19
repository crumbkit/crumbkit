# ADR-0091. One row in the singular

- Date: 2026-10-19
- Status: accepted
- Supersedes: ADR-0087, which it amends
- Missed: ADR-0087 dropped the count from an uncounted listing's footer and cut line, so the line put the number of shown rows straight before the plural description, and one row read `Showing 1 entries`.

## Context

A listing's description is a plural phrase that the tool supplies, such as
`books` or `books by author 4`. A counted listing's footer reads `Showing 1
of 6 books.`, where the plural is correct. After ADR-0087, an uncounted
listing's footer and cut line read `Showing {shown} {description}`, so one
shown row read `Showing 1 entries, and more match.`, as the reading
example's `entries list --limit 1` printed. crumbkit cannot derive the
singular of a phrase, so only the tool can give it.

## Decision

- `Listing::singular` sets the phrase that names one row, such as `book`.
  The footer and the cut line of a listing without a total use it when they
  show one row, as in `Showing 1 book, and more match.`
- An uncounted listing without a description names one row as `row`.
- A listing with a description and no singular names one row with its
  description, as before, and a counted listing never uses the singular.
- The JSON document is unchanged.

## Consequences

A tool with an uncounted listing sets the singular beside its description,
as the reading example does. The singular is a `String`,
as the description is, because a tool may build both from its arguments.
`docs/behaviour/output/listings/README.md` describes the current
behaviour.

Considered: rewording the line as `Showing 1 of the entries`, which needs
no singular, deriving the singular by trimming an English plural ending,
a `described` that takes both forms, which changes every existing caller.
