# ADR-0017. Decision records

- Date: 2026-10-14
- Status: accepted

## Context

Until this date, each record stated a decision as crumbkit held it at the
time, and a changed decision was edited in place because git history kept
the old version. The records therefore held both the current behaviour and
the reasons for it, and the rustdoc held a second copy of much of that
behaviour. A reader could not tell which copy was right, and a record could
not show what the project had believed before a change.

Records already formed a tree with global numbers. Every directory's
`README.md` was a node record that linked to its children, and a directory
near the 5,000-word limit of DOCS-2 split by topic into child directories.

## Decision

- A decision record became a dated record of one decision: its context, the
  decision, its consequences and the alternatives considered. A record
  starts with `Date` and `Status` lines, and a record that replaces or
  amends an earlier one adds a `Supersedes` line that names it. The pages
  in `docs/` and the rustdoc state what crumbkit does now (ADR-0013).
- Every record that existed on this date, ADR-0001 to ADR-0057, was
  rewritten once, in the switch to this format, and trimmed to its context
  and reasons. Each kept its number, path, title and Considered line, and
  each is accepted as of the day it was first added. The current behaviour
  that the old records described moved to the pages in `docs/`.
- A record is frozen once it has a `Date` line on `main`. A later decision
  gets a new record whose `Supersedes` line names the record it replaces or
  amends, and the earlier record's `Status` line changes to
  `superseded by ADR-NNNN` or `amended by ADR-NNNN`. The Status line and a
  node's `## Children` list are the only parts of a frozen record that may
  change (DOCS-6).
- Records still form a tree. Every directory's `README.md` is a node record
  that states which decisions belong under it and lists its children, so
  every record can be reached from the root. A node needs no Considered
  line, because its scope is its only decision. When a directory nears the
  5,000-word limit of DOCS-2, its records move by topic into child
  directories, and each new directory's `README.md` takes the next free
  number.
- A record's number is global and never reused, and moving a record keeps
  its number. Text anywhere in the repository cites a record by that number
  (DOCS-3), so one search finds every reference to a decision. Text outside
  `docs/decisions/` may not cite a superseded record.

## Consequences

- `cargo xtask check` compares each record with its version at the merge
  base with `main` and fails on any edit other than the Status line or a
  node's Children list, as `docs/project/checks.md` describes.
- Changing a decision costs a new record, which is longer than an edit in
  place but keeps the old decision readable.
- A reader finds current behaviour in `docs/`, whose pages cite records
  only for their reasons. `docs/project/documentation.md` names the home
  of each kind of fact.

Considered: a flat directory, numbers local to each directory, citing
records by file path, freezing the existing records without a rewrite, a
dated note on each existing record, a flat directory in date order for new
records, editing records in place as before.

## Children

- [ADR-0018. API](api/README.md)
- [ADR-0019. Testing](testing/README.md)
- [ADR-0020. Project](project/README.md)
