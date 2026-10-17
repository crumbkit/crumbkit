# ADR-0069. Output budget floor

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0046, which it amends

## Context

`Guide::output_budget` panicked on a budget below 1024 bytes (ADR-0046). A
tool's in-process tests build their guide with that method, so a test that
wanted to show a cut result had to print more than 1024 bytes of rows. Small
test data could not reach the cut.

The floor did not protect a reader. `CRUMBKIT_OUTPUT_BUDGET` already lowers
the budget of one run to any value of 1 byte or more, and a result that the
budget refuses prints its over-budget failure whatever the failure's size.

## Decision

- `Guide::output_budget` accepts any budget of at least 1 byte.
- A budget of 0 bytes panics, because a run with it could print no answer.
  A `const` guide reports the panic when the tool builds.
- The constant `budget::SMALLEST`, which held the 1024-byte floor, is
  removed.

## Consequences

A tool's tests can set a budget of a few hundred bytes and show a cut
listing or batch with a few short rows. A tool that ships a small budget
gets results that are cut early, and the opt-out report shows the budget
and its reason as before.

Considered: a budget builder on `Output` for tests alone, keeping the
1024-byte floor.
