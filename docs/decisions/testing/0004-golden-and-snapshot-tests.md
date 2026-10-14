# ADR-0004. Golden files

- Date: 2026-10-07
- Status: accepted

## Context

Much of crumbkit's behaviour is the exact text a tool prints: help pages,
example output and reports. A test of that text must show any change in
review and must be cheap to update when a change is intended.

## Decision

- snapbox checked golden files, which hold the exact bytes a tool printed,
  at paths that crumbkit chooses under `tests/golden/`.
- Contributors update golden files with `SNAPSHOTS=overwrite` and review
  the result with `git diff`. CI never writes them, so a change to the
  output always arrives in a reviewed commit.

## Consequences

A golden file of example output may hold named placeholders for values
that change between runs, and snapbox's wildcards are plain text in it.
`docs/project/testing.md` lists the current golden directories.

Considered: insta, expect-test, plain `assert_eq!` files.
