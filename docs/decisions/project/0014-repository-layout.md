# ADR-0014. Repository layout

- Date: 2026-10-07
- Status: accepted

## Context

crumbkit needed a layout for several published crates, example tools
that exercise them, benchmarks and repository checks, and a rule for
where modules and tests live.

## Decision

- crumbkit became a virtual workspace, with published crates under
  `crates/` and unpublished packages for the examples, the benchmarks and
  `xtask`.
- `Cargo.lock` was committed, and published crates list their files in
  `include`.
- Modules and unit tests followed RUST-6 and RUST-7, so each file name
  says which module it holds and code and tests read separately.

## Consequences

Each new crate or example is a workspace member. `docs/project/README.md`
lists the current crates and packages.

Considered: a root package, shelf as a crate example, inline test modules.
