# ADR-0009. Lints

- Date: 2026-10-07
- Status: accepted

## Context

crumbkit needed a lint policy that keeps the build free of warnings
without letting a new compiler release break unrelated changes, and that
keeps side effects out of code that tests should run in memory.

## Decision

- Every lint except `unsafe_code` was set to warn in
  `[workspace.lints]`, and CI turned warnings into failures through
  `CARGO_BUILD_WARNINGS=deny`, because a warning left in the build hides
  the next one.
- `unsafe_code` was forbidden, because crumbkit needs no `unsafe` and the
  ban rules out memory-safety bugs in crumbkit's own code.
- A suppressed lint carries `#[expect]` and a reason, because the reason
  records why and `#[expect]` warns when the lint stops firing.
- `rust-toolchain.toml` pinned the stable toolchain, so a new clippy
  release never fails an unrelated change.
- `clippy.toml` confined each effect to the module that owns it, so the
  rest of the code takes a writer or a value and its tests run in memory.

## Consequences

Each rule became a standard with an ID (RUST-1 to RUST-4 and RUST-9). A
toolchain update is a deliberate change of its own. The ban on unsafe code
later decided several limits of the usage log and the Windows support.

Considered: `#![deny(warnings)]`, `RUSTFLAGS="-D warnings"`, cargo-spellcheck, an unpinned stable toolchain, an xtask suppression check.
