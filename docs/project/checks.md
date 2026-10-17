# Checks

This page describes the lints, the repository checks in `xtask` and the
supply-chain checks. [docs/standards](../standards/README.md) lists each
rule with its ID and the gate that enforces it, and [CI](ci.md) says which
job runs each check.

## Lints

- `[workspace.lints]` sets every lint except `unsafe_code` to warn, and CI
  turns each warning into a failure through `CARGO_BUILD_WARNINGS=deny`
  (RUST-1), because a warning left in the build hides the next one.
- rustc's `unsafe_code` lint is forbidden (RUST-2), because crumbkit needs
  no `unsafe` and forbidding it rules out memory-safety bugs in crumbkit's
  own code.
- A suppressed lint carries `#[expect]` and a reason (RUST-3). The reason
  records why, and `#[expect]` warns when the lint stops firing.
- `clippy.toml` confines each effect to the module that owns it (RUST-4),
  so the rest of the code takes a writer or a value and its tests run in
  memory.
- `rust-toolchain.toml` pins the stable toolchain (RUST-9), so a new clippy
  release never fails an unrelated change.

Why: ADR-0009.

## Repository checks

The unpublished `xtask` crate runs the repository checks, and a `#[test]`
calls each one. xtask parses its arguments with clap. Its rustdoc in
[`xtask/src/main.rs`](../../xtask/src/main.rs) lists the commands.

- `cargo xtask check` enforces the line limit of RUST-8, so a person or an
  agent can read each `.rs` file whole, and the word limit of DOCS-1 on
  `AGENTS.md`, because an agent reads that file at the start of every
  session. It lists files with `git ls-files`, including untracked files
  that are not ignored. It also runs the docs checks and the docs trailer
  check of DOCS-7, which reads the commit messages after the merge base
  with `origin/main`.
- `cargo xtask check docs` runs only the docs checks: the directory word
  limit of DOCS-2, so each directory of the docs tree can be read in one
  sitting, the record checks of DOCS-3, and the frozen record check of
  DOCS-6, which compares each record with its text at the merge base with
  `origin/main`. They also run the missed line check of DOCS-8, which reads
  each record that is new since the merge base and fails when it amends a
  record dated less than 7 days before it without a `- Missed:` line. Both commands fail with an error that says so when the
  clone has no `origin/main`, such as a shallow clone. The tree includes `docs/`
  itself and every subdirectory, and each directory's count includes only
  the files directly in it.
- `cargo xtask ci-time` measures the CI-3 limit from the run times that
  `gh api` returns for `ci.yml`.
- `cargo xtask run-time` builds shelf in release mode and times its whole
  runs against the PERF-2 limit, with an empty environment and XDG
  directories under `target/run-time/`. [Testing](testing.md) states the
  limits.

Why: ADR-0010, ADR-0017, ADR-0074.

## Supply-chain checks

cargo-deny's licence, ban and source checks and cargo-shear enforce CI-2 in
the pull-request `lint` job, because they take seconds and stop a bad
dependency before it merges. An unexpected licence binds adopters, an
unknown source can serve code that nobody reviewed, and an unused
dependency adds build time for nothing.

`deny.toml` allows only the licences the graph uses and only crates.io as a
source. It bans these:

- duplicate versions, and wildcard version requirements except a
  path-only dev-dependency, which `cargo publish` leaves out;
- the async runtimes and C build tools that
  [Dependencies](dependencies.md#admitting-a-dependency) rules out;
- clap in any crate that its list of clap users leaves out, which keeps
  clap out of crumbkit-core;
- an executable or a script inside a crate that runs at compile time,
  except the helper scripts that `deny.toml` names with the reason no build
  runs them.

cargo-deny's advisories check runs in the nightly lane, because a new
advisory can appear when no code has changed.

Dependabot checks the cargo and GitHub Actions dependencies weekly and puts
each ecosystem's version updates in one pull request. It proposes a release
only after a 7-day cooldown, so the ecosystem has time to find and yank a
compromised release.

Why: ADR-0012, ADR-0003, ADR-0032.
