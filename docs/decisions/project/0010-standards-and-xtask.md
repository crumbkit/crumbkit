# ADR-0010. Standards and xtask

- Date: 2026-10-07
- Status: accepted

## Context

crumbkit's rules for its own code and docs needed a single list, and each
rule needed a gate that enforces it. Some gates, such as line and word
limits, had no off-the-shelf tool.

## Decision

- Each rule in `docs/standards/` got an ID, such as `RUST-4`, and named
  the gate that enforces it.
- An unpublished `xtask` crate ran the repository checks, with a `#[test]`
  for each, and parsed its arguments with clap. The checks were written in Rust in place of shell scripts or a task
  runner.
- `cargo xtask check` enforced the line limit of RUST-8, so a person or an
  agent can read each `.rs` file whole, and the word limit of DOCS-1,
  because an agent reads an `AGENTS.md` at the start of every session.
- `cargo xtask check docs` enforced the directory word limit of DOCS-2, so
  each directory of the docs tree can be read in one sitting, and the
  record checks of DOCS-3.
- `cargo xtask ci-time` and `cargo xtask run-time` measured the CI-3 and
  PERF-2 time limits.

## Consequences

A new repository rule is a module of xtask with its tests, and it runs in
CI through `cargo xtask check`. `docs/project/checks.md` lists the current
commands.

Considered: shell scripts, just, cargo-make, an argument parser written by hand.
