# Testing

This page describes how crumbkit tests itself. [Checks for
tools](../behaviour/checks/README.md) describes the checks that
crumbkit-test gives a tool, which crumbkit also runs on shelf and reading.

## Golden files

snapbox checks golden files, which hold the exact bytes a tool printed, at
paths under `tests/golden/`. Contributors update them with
`SNAPSHOTS=overwrite` and review the result with `git diff`. CI never
writes them. [Golden files](../behaviour/checks/golden-files.md) lists the
files that each check keeps.

Why: ADR-0004.

## End-to-end tests

End-to-end tests run the built binary through snapbox's `cmd` module with a
cleared environment and temporary XDG directories. crumbkit-test's own
checks run the binary through `std::process` with their own timeout, as
[End-to-end checks](../behaviour/checks/end-to-end.md#timeouts-and-pipes)
describes.

Why: ADR-0005.

## Property tests and fuzzing

proptest checks the code segment rules, the document component rule, the
predefined spellings, table rendering and the reduction of usage records.
The reduction must report the same figures and findings for any
interleaving of sessions in the log and for a log read in two parts.

The fast lane runs the property tests briefly with the fixed seed of
RUST-11, so a CI run generates the same cases on every machine. proptest
writes no regression files, so the unit test that RUST-11 asks a fix to add
keeps the failing input.

The usage-log reader's bolero fuzz target arrives with the change that sets
up a nightly fuzz lane, which will fuzz each target for 10 minutes.

Why: ADR-0006.

## Performance budgets

gungraun benchmarks in the unpublished `benches/` package run shelf's whole
path in-process through `shelf::run_from`: `books list`, root and leaf help,
`commands list --all --json --full`, a refusal, and `books list` with the
usage log.

- Each benchmark has a limit on Callgrind's instruction count and on DHAT's
  count of allocated blocks, because both counts stay the same from run to
  run on shared CI machines, where wall time does not.
- DHAT records 500 stack frames for each allocation, valgrind's largest.
  gungraun counts only allocations whose recorded stack reaches the
  benchmark function, and the default of 12 frames left deeper allocations
  out of the count.
- Each limit is the count of a nightly run plus 10 percent, rounded up, and
  sits beside a comment that gives the count. When the nightly job fails,
  the fix or a raised limit comes in a pull request whose description cites
  that run's counts.
- gungraun's version is exact and Dependabot leaves it alone, because the
  runner binary must match it and a new version can change the counts.

`cargo xtask run-time` times 200 whole runs of the release build of shelf
for each of `books list` and `commands list --all --json --full`, after 20
warm-up runs. A median over 10 ms prints a warning, and a median over 30 ms
fails the nightly `bench` job.

The benchmarks run in the nightly `bench` job on the pinned `ubuntu-24.04`
image, and a run over a limit fails the job. They stay out of pull
requests, so pull-request CI stays fast and a regression shows up the next
morning. Help pages stay within the byte budgets that
[Help checks](../behaviour/help/checks.md) describes.

Why: ADR-0008, ADR-0025.
