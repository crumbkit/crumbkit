# CI

crumbkit's CI has three workflows: `ci.yml` for pull requests and pushes to
`main`, `scheduled.yml` for the nightly lane, and `release.yml` for
releases. The workflows follow CI-1, because a broad token or a movable tag
lets a compromised action change the repository or run code that nobody
reviewed. Every Rust job runs on a GitHub image that includes rustup, and
`rustup toolchain install` installs the toolchain that
`rust-toolchain.toml` pins.

## The pull-request lane

The fast lane's median wall time stays within the limit of CI-3, because a
slow pull-request run delays every change. It has two work jobs, `lane` and
`crumbkit`, which run in parallel.

- `lane` calls `rust-lane.yml` from crumbkit/rust-standards at commit
  5ba71ff, with `library: true` and `timeout-minutes: 10`, and shows as
  `lane / lane`. Its one job on `ubuntu-24.04` runs rustfmt, zizmor,
  gitleaks, cargo-deny, cargo-shear, clippy with every feature, the tests
  with cargo-nextest's default profile, the doctests, rustdoc with warnings
  denied (DOCS-4), and clippy on each feature alone. A hand-written pull
  request moves the commit, and Dependabot ignores the repository.
- `crumbkit` runs on `ubuntu-latest` the steps the shared lane lacks: the
  commit check on a pull request, the MSRV check of RUST-9,
  `cargo xtask check`, `cargo doc -p crumbkit` and the per-package runs.
- `gate` needs `[lane, crumbkit]`, runs even when one of them failed or was
  cancelled, and fails unless both succeeded. The ruleset requires `gate`
  alone, so a job that joins the fast lane needs no change to the ruleset.
  `gate` runs on `ubuntu-slim`, because it only reads results.

`ci.yml` also runs `lane` and `crumbkit` on each push to `main`, so the
merged code is tested even when `main` moved after a pull request's last
run. A newer push to `main` cancels the run of an older one. The repository
has no merge queue, so `ci.yml` has no `merge_group` trigger. On each push
to `main`, the `macos` job runs the tests on `macos-latest`. It stays out of
pull requests and the gate, because GitHub runs only a few macOS jobs of an
account at once, so a pull request would wait for a runner.

The `lane / lane`, `crumbkit` and `macos` jobs use rust-cache, and only a run
on `main` saves a cache. A pull request restores the cache that `main` saved,
so pull requests skip the time of saving and leave the cache storage to
`main`.

Why: ADR-0011, ADR-0260.

## Feature builds

shelf turns on crumbkit's `miette` and `tracing` features and uses
crumbkit-usage, and reading uses crumbkit-child, so the workspace runs
build, test and lint crumbkit with both features, the usage log and
crumbkit-child. crumbkit-usage's Unix dev-dependency turns on
crumbkit-process's `forward` feature, so the workspace runs on Unix build,
test and lint crumbkit-process with it.

- `crumbkit` runs crumbkit's tests and doctests again in the default build
  and with `miette`.
- `crumbkit` runs crumbkit-process's tests alone with `-p`. Cargo unifies
  features only across the selected packages, so that run builds the crate
  without `forward`, which is an adopter's default build.
- The lane's `features` step runs clippy on each feature of each package
  alone, which covers crumbkit with `tracing` alone, the only build that
  compiles the spans without the usage log's ids.
- Only the nightly `windows` job runs clippy on the default build, where
  shelf enables `miette` and `tracing`.

A separate run for crumbkit-usage or crumbkit-child would repeat what the
workspace runs check. crumbkit-child's own tests run with no usage log, so
they cover the child's SIGINT handler without the log's signal thread.
Clippy runs once per build, because a single `--all-features`
run misses a warning that only one build shows.

Why: ADR-0011, ADR-0260.

## The commit check

On a pull request, `crumbkit` first runs `.github/scripts/commit-check.sh`.
The script fails when the author name, author email, committer name,
committer email or message of any commit between the pull request's base
and head matches the case-insensitive extended regular expression in the
repository secret `COMMIT_PATTERN`, which the owner sets.

- The check runs in `crumbkit`, so `gate` blocks the merge without a change
  to the ruleset, and `crumbkit` checks out the full history so every commit
  is present.
- On a match the script prints the commit's SHA and the name of the field,
  and never the field's value, so the log does not reveal what matched.
- The workflow passes the secret through `env:`, because an expression
  written inside `run:` becomes part of the shell script's text.
- When the secret is empty the check passes with a notice. A pull request
  from a fork gets no repository secrets, and one that Dependabot opens gets
  only Dependabot secrets, so both skip the check unless the owner also sets
  `COMMIT_PATTERN` as a Dependabot secret.

Why: ADR-0011, ADR-0260.

## The ruleset

The repository ruleset `main` requires the `gate` check to pass before a
pull request merges, requires every change to `main` to come through a pull
request, and blocks force pushes to `main` and its deletion. It does not
require a branch to be up to date with `main`, because the run on each push
to `main` tests the merged code. It requires no approving review, because
the owner cannot approve their own pull requests.

Why: ADR-0011.

## The scheduled lane

`scheduled.yml` runs every night.

- `nightly` runs on `ubuntu-latest`. It runs `cargo deny check advisories`
  and `cargo xtask ci-time`, the gate of CI-3. It also builds the published
  crates' docs as docs.rs does, on the nightly toolchain with every feature
  and `--cfg docsrs`, in a target directory of its own, and fails on a
  rustdoc warning. These run every night, because a new advisory or a new
  nightly toolchain needs no change on `main`.
- `bench` runs the benchmarks and `cargo xtask run-time`, the gates of
  PERF-1 and PERF-2, which [Testing](testing.md#performance-budgets)
  describes.
- `changed` runs on `ubuntu-slim` and finds whether the newest of the last
  ten completed runs on `main` whose `windows` job passed ran at the same
  commit. `.github/scripts/last-passed.sh` holds that lookup for `bench` and
  `windows`.
- `windows` builds, lints and tests the workspace on `windows-latest`, the
  gate of CI-4. It runs clippy on the workspace, the tests with
  cargo-nextest and the doctests. crumbkit-child's tests in the workspace
  run cover the Windows SIGINT and SIGBREAK handlers without the usage
  log, and crumbkit-process's Windows tests cover its job object. It turns off debug info and
  incremental builds.
- `clap-latest` tests crumbkit and shelf on the newest clap and
  clap_complete that `Cargo.toml` allows, ignoring `rust-version`.
- `report` lists a failed run's URL and jobs on the "Scheduled CI failed"
  issue, or opens it.

A scheduled run skips `bench` and `windows` when the last passing run of
that job was at the same commit, because nothing they check has changed. A
manual run always runs them. No scheduled job uses rust-cache, because
rust-cache keys include the job ID, so those caches would not help the
pull-request jobs.

Why: ADR-0011, ADR-0006, ADR-0008.

## The release lane

On each push to `main`, the `pull-request` job of `release.yml` runs
release-plz to open or update the release pull request, and its `publish`
job runs release-plz to publish what that pull request set. Only `publish`
gets `id-token: write` and the `crates-io` environment. Neither job uses
rust-cache, because a poisoned cache would reach the published crates, and
zizmor fails a release workflow that restores one.
[Releases](releases.md) describes the process.

Why: ADR-0011, ADR-0012.
