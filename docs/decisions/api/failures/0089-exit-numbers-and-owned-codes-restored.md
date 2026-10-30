# ADR-0089. Exit numbers and owned codes restored

- Date: 2026-10-18
- Status: amended by ADR-0160
- Supersedes: ADR-0085 and ADR-0088, which it amends
- Missed: ADR-0085's review counted only the callers inside this repository. A tool outside it calls `Stop::number`, `Recoveries::from_codes`, `Twins::from_codes` and `Unprovoked::from_code`, so removing them broke that tool. ADR-0088 kept the three crumbkit-test constructors removed, because it missed that the tool builds its codes at run time and holds them by value.

## Context

ADR-0085 made `Stop::number` crate-private, so only the run's trace span
and crumbkit's tests read it. It also removed `Recoveries::from_codes`,
`Twins::from_codes` and `Unprovoked::from_code` from crumbkit-test.
ADR-0088 restored other items that a tool outside this repository calls,
and kept the three crumbkit-test constructors removed.

A tool outside this repository records the exit code of a run that `parse`
stopped in a log of its own and passes it on. An `ExitCode` cannot be read
back as a number. Without `Stop::number`, the tool compares `Stop::code`
with `ExitCode::from(0)` and `ExitCode::from(2)` to guess the number, and
the guess is wrong when the guide lists another exit for a refusal.

The same tool builds some of its codes at run time with `Code::runtime` and
holds them by value in its tests. `Recoveries::new` and `Twins::new` take
a slice of lists of references to codes, so with them the tool collects a
`Vec<&Code>` from its owned codes and wraps it in an outer slice before
each check. The `from_codes` and `from_code` constructors take the owned
codes directly and save that step.

API-1 covers `Document` options, `Row` constants, `Guide` settings and
`Leaf` settings. None of the restored items is one of those, so API-1 does
not decide them.

## Decision

- `Stop::number` is public again and returns the exit code that
  `Stop::code` returns, as a `u8`.
- In crumbkit-test, `Recoveries::from_codes(examples, codes)` and
  `Twins::from_codes(examples, codes)` take one slice of codes held by
  value, and `Unprovoked::from_code(code, reason)` takes one code by value.
  They take no spelling, because `check_recoveries` and `check_twins` read
  the spelling from the guide, as `new` does since ADR-0086.
- Every other removal of ADR-0085 that ADR-0088 kept stays.

## Consequences

The compile_fail doctests in crumbkit's and crumbkit-test's `removed.rs`
no longer hold the restored items. Inside this repository, the uses of
`Stop::number` are the trace span and the tests, including a test outside
the crate that calls it as a tool does. The uses of the three crumbkit-test
constructors are crumbkit-test's own unit tests.

Considered: asking the outside tool to map each `ExitCode` it can return
to a number itself, which repeats the guide's exits in the tool and breaks
when the guide changes; keeping the reference constructors only, which
works but makes a tool with owned codes collect references and wrap them
in an outer slice for each check.
