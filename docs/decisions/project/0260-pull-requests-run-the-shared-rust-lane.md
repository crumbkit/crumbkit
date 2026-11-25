# ADR-0260. Pull requests run the shared Rust lane

- Date: 2026-11-25
- Status: accepted
- Supersedes: ADR-0011, which it amends

## Context

ADR-0011 gave pull requests two work jobs, `lint` and `test`, and a `gate`
job that needs both. Their tools, zizmor, cargo-deny, cargo-shear and
cargo-nextest, installed at whatever version install-action chose.
crumbkit/rust-standards#10 gives every Rust repository one shared lane,
`rust-lane.yml`, with one set of tool pins, so a fix to a step or a pin
reaches every repository from one place. The shared lane runs rustfmt,
zizmor, gitleaks, cargo-deny, cargo-shear, clippy with every feature, the
tests and doctests with every feature, and, for a library, rustdoc and a
clippy run on each feature alone. It lacks crumbkit's commit check, MSRV
check, `cargo xtask check`, rustdoc on crumbkit's default build and the
per-package test runs. The repository ruleset requires the check `gate`.

## Decision

These bullets amend ADR-0011's bullets on `lint`, `test`, `gate`, the commit
check and the feature builds.

- A `lane` job calls `rust-lane.yml` at a full commit of
  crumbkit/rust-standards, with `library: true` and `timeout-minutes: 10`.
  A pull request shows its check as `lane / lane`.
- A `crumbkit` job runs the steps the shared lane lacks: the commit check,
  the MSRV check, `cargo xtask check`, rustdoc on crumbkit's default build,
  the per-package tests of crumbkit and crumbkit-process, and crumbkit's
  doctests.
- `gate` needs `[lane, crumbkit]` and stays the check that the ruleset
  requires, so the ruleset needs no change. A caller job named `gate` would
  report its check as `gate / lane`, which the ruleset does not name, so
  `gate` stays a separate job that reads the results.
- The commit check moves to `crumbkit`, and its rules from ADR-0011 stay as
  they are.
- The shared lane's `features` step runs clippy on each feature of each
  package alone, which replaces the per-feature and per-package clippy runs.
- `macos` and `scheduled.yml` use the shared lane's pins of install-action,
  cargo-nextest and cargo-deny.
- A hand-written pull request moves the shared lane's commit, and
  Dependabot ignores crumbkit/rust-standards, so a new commit of the shared
  lane never reaches crumbkit without review.

ADR-0011's bullets on the scheduled lane, macOS, rust-cache and CI-1 stay as
they are.

## Consequences

A pull request runs gitleaks on its commits, `--locked` on every cargo step
of the shared lane, and clippy with every feature, which it did not run
before. The shared lane is one job where `lint` and `test` were two, so a
run may take longer, and CI-3 measures it. `docs/project/ci.md` lists the
current jobs and steps.

Considered: a caller job named `gate`, keeping `lint` and `test` beside the
shared lane, and letting Dependabot move the shared lane's commit.
