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
slow pull-request run delays every change. It has two work jobs, `lint` and
`test`, which run in parallel on `ubuntu-latest`. GitHub's standard runners
are free for a public repository, and `ubuntu-latest` has 4 CPUs, so cargo
builds and tests in parallel, which the single CPU of `ubuntu-slim` cannot.
Every extra job repeats the checkout, the toolchain installation and the
cache restore, so a third work job would shorten a run only by the steps it
took from the longer job.

- `lint` runs the commit check on a pull request, rustfmt, clippy, the MSRV
  check of RUST-9, `cargo xtask check`, rustdoc with warnings denied
  (DOCS-4), cargo-deny, cargo-shear and zizmor.
- `test` runs the tests with cargo-nextest, whose `ci` profile reports every
  failing test in one run, and the doctests with `cargo test --doc`,
  because nextest does not run doctests.
- `gate` needs every fast-lane job, runs even when one of them failed or was
  cancelled, and fails unless all of them succeeded. The ruleset requires
  `gate` alone, so a job that joins the fast lane needs no change to the
  ruleset. `gate` runs on `ubuntu-slim`, because it only reads results.

`ci.yml` also runs `lint` and `test` on each push to `main`, so the merged
code is tested even when `main` moved after a pull request's last run. A
newer push to `main` cancels the run of an older one. The repository has no
merge queue, so `ci.yml` has no `merge_group` trigger. On each push to
`main`, the `macos` job runs the tests on `macos-latest`. It stays out of
pull requests and the gate, because GitHub runs only a few macOS jobs of an
account at once, so a pull request would wait for a runner.

The `lint`, `test` and `macos` jobs use rust-cache, and only a run on `main`
saves a cache. A pull request restores the cache that `main` saved, so pull
requests skip the time of saving and leave the cache storage to `main`.

Why: ADR-0011.

## Feature builds

shelf turns on crumbkit's `miette`, `usage-log` and `tracing` features, and
reading turns on `hand-over`, so the workspace runs build, test and lint
crumbkit with all four.

- `test` runs crumbkit's tests and doctests again in the default build and
  with `miette`, and its tests with `hand-over` alone.
- `lint` runs clippy on crumbkit in the default build, with `miette`, with
  `tracing` alone, which is the only build that compiles the spans without
  the usage log's ids, and with `hand-over` alone, which is the only build
  that installs the child's SIGINT handler without the usage log.

A separate `usage-log` run would repeat what the workspace runs check. Each
build is one more step in an existing job, because a new job repeats the
setup time. Clippy runs once per build, because a single `--all-features`
run misses a warning that only one build shows.

Why: ADR-0011.

## The commit check

On a pull request, `lint` first runs `.github/scripts/commit-check.sh`. The
script fails when the author name, author email, committer name, committer
email or message of any commit between the pull request's base and head
matches the case-insensitive extended regular expression in the repository
secret `COMMIT_PATTERN`, which the owner sets.

- The check runs in `lint`, so `gate` blocks the merge without a change to
  the ruleset, and `lint` checks out the full history so every commit is
  present.
- On a match the script prints the commit's SHA and the name of the field,
  and never the field's value, so the log does not reveal what matched.
- The workflow passes the secret through `env:`, because an expression
  written inside `run:` becomes part of the shell script's text.
- When the secret is empty the check passes with a notice. A pull request
  from a fork gets no repository secrets, and one that Dependabot opens gets
  only Dependabot secrets, so both skip the check unless the owner also sets
  `COMMIT_PATTERN` as a Dependabot secret.

Why: ADR-0011.

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
  gate of CI-4. It runs clippy on the workspace and on crumbkit with
  `hand-over` alone, the tests with cargo-nextest, the doctests, and
  crumbkit's tests with `hand-over` alone, which cover the Windows SIGINT
  and SIGBREAK handlers without the usage log. It turns off debug info and
  incremental builds.
- `report` comments the run's URL on the open issue titled "Scheduled CI
  failed" when `nightly`, `bench`, `changed` or `windows` fails, or opens
  that issue when none is open.

A scheduled run skips `bench` and `windows` when the last passing run of
that job was at the same commit, because nothing they check has changed. A
manual run always runs them. No scheduled job uses rust-cache, because
rust-cache keys include the job ID, so those caches would not help the
pull-request jobs. Fuzzing joins the scheduled lane once code for it exists.

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
