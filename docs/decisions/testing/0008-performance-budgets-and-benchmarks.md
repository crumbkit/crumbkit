# ADR-0008. Performance budgets

- Date: 2026-10-08
- Status: accepted

## Context

A CLI runs once per command, so its start-up and whole-run cost matter on
every call an agent makes. Wall time on shared CI machines varies from run
to run, so a wall-time limit there is noisy.

## Decision

- gungraun benchmarks ran shelf's whole path in-process, with limits on
  Callgrind's instruction count and on DHAT's count of allocated blocks,
  because both counts stay the same from run to run on shared machines.
  DHAT records the largest stack depth, because gungraun counts only
  allocations whose stack reaches the benchmark function.
- The benchmarks ran nightly and stayed out of pull requests, because the
  owner keeps pull-request CI fast and accepts that a regression shows up
  the next morning.
- Each limit was a nightly count plus 10 percent, with the count in a
  comment. A fix or a raised limit cites the failing run's counts.
  gungraun's version was pinned exactly, because the runner binary must
  match it and a new version can change the counts.
- `cargo xtask run-time` timed whole release runs of shelf, with a warning
  over a 10 ms median and a failure over 30 ms.

## Consequences

A performance regression fails the nightly run and is recorded on the
scheduled issue. `docs/project/testing.md` lists the current benchmarks.

Considered: criterion, divan, counting allocators, the fast lane on every pull request, only on release pull requests, gungraun binary benchmarks, hyperfine.
