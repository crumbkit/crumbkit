# ADR-0009. Lints

- `[workspace.lints]` sets every lint except `unsafe_code` to warn, and CI turns each warning into a failure through `CARGO_BUILD_WARNINGS=deny` (RUST-1), because a warning left in the build hides the next one.
- rustc's `unsafe_code` lint is forbidden (RUST-2), because crumbkit needs no `unsafe` and forbidding it rules out memory-safety bugs in crumbkit's own code.
- A suppressed lint carries `#[expect]` and a reason (RUST-3), because the reason records why, and `#[expect]` warns when the lint stops firing.
- `rust-toolchain.toml` pins the stable toolchain (RUST-9), so a new clippy release never fails an unrelated change.
- `clippy.toml` confines each effect to the module that owns it (RUST-4), so the rest of the code takes a writer or a value and its tests run in memory.

Considered: `#![deny(warnings)]`, `RUSTFLAGS="-D warnings"`, cargo-spellcheck, an unpinned stable toolchain, an xtask suppression check.
