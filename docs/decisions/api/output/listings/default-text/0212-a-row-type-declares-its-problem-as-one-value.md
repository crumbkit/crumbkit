# ADR-0212. A row type declares its problem as one value

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0096, which it amends

## Context

Under ADR-0096 a row type said which rows report a problem in two places.
`Row::PROBLEM` named a field, and `Row::is_problem` decided, with a default
that read that field. The two could disagree. A type that overrode
`is_problem` and also set `PROBLEM` printed a `Problems:` help line, and
`check_help` checked a field that no longer decided anything. A type that
overrode `is_problem` named no column, so nothing checked that the compact
table showed the state that decides the exit.

## Decision

- `Row::PROBLEM` holds an `Option<Problem<Self>>`, and `Row::is_problem` is
  removed. `Problem` has two constructors.
- `Problem::field("status")` marks a row whose field serializes to a value
  other than `null`, `false` or an empty string, as the default of
  `is_problem` did.
- `Problem::by("state", Self::failed)` lets the function decide, and names
  the column that shows the state.
- crumbkit-test's `check_help` fails either kind whose column is not one of
  `Row::COLUMNS`. It checks the column's type only for `Problem::field`.
- The `Problems:` help line prints only for `Problem::field`, as before, so
  help, `commands list`, the exit code and the note do not change.
- `Problem` holds a function pointer, so it has `Clone`, `Copy` and `Debug`
  written without a bound on the row type, and no `PartialEq`.

## Consequences

A row type cannot set one half of its problem without the other, and a
computed problem names the column that shows it. A row type that overrode
`is_problem` moves its body into a function or a closure that it passes to
`Problem::by`. A caller outside crumbkit that asked a row whether it
reports a problem writes `R::PROBLEM.is_some_and(|problem|
problem.reports(row))`. `docs/behaviour/output/listings/README.md`
describes the current behaviour.

Considered: a `Problems:` help line for `Problem::by`, which can follow
without a break; leaving the column of `Problem::by` unchecked, which keeps
the gap that ADR-0096 left; and a public helper that asks a row whether it
reports a problem, which waits for an adopter that wants one.
