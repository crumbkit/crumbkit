# ADR-0006. Property tests and fuzzing

- Date: 2026-10-07
- Status: accepted

## Context

Some of crumbkit's rules, such as code and component spellings, table
rendering and the reduction of usage records, must hold for any input,
which example-based tests cannot cover. A property test in CI must also
generate the same cases on every machine.

## Decision

- proptest checked those rules. The usage log's reduction must report the
  same figures for any interleaving of sessions and for a log read in two
  parts.
- The fast lane ran the property tests briefly with the fixed seed of
  RUST-11, so a CI run is repeatable. proptest writes no regression files,
  so a fix adds a unit test that keeps the failing input.
- Fuzzing with bolero was planned for a nightly lane, each target for 10
  minutes, starting with the usage-log reader.

## Consequences

When this record was written, the fuzz target and its nightly job did not
exist. They arrive with the change that sets up the fuzz lane.

Considered: cargo-fuzz, quickcheck.
