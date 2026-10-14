# ADR-0011. CI lanes

- Date: 2026-10-07
- Status: accepted

## Context

Every change to crumbkit waits for the pull-request run, so the time of
that run is a cost paid on each change. GitHub's standard runners are free
for a public repository, but each job repeats the checkout, the toolchain
installation and the cache restore. Some checks, such as new advisories,
benchmarks, Windows and the docs.rs build, matter without a change on
`main` or only at a release. GitHub runs only a few macOS jobs of an
account at once.

## Decision

- Pull requests ran a fast lane of two parallel work jobs, `lint` and
  `test`, on `ubuntu-latest`. `ubuntu-latest` has 4 CPUs, so cargo builds
  and tests in parallel, which the single CPU of `ubuntu-slim` cannot. Two
  jobs were chosen because a third job would shorten a run only by the
  steps it took from the longer job, while repeating the setup.
- A `gate` job needed every fast-lane job and failed unless all of them
  succeeded, so the ruleset requires `gate` alone and a new fast-lane job
  needs no ruleset change. The ruleset did not require a branch to be up
  to date with `main`, because `ci.yml` also runs on each push to `main`
  and so tests the merged code. It required no approving review, because
  the owner cannot approve their own pull requests.
- `lint` ran a commit check against the repository secret
  `COMMIT_PATTERN` on each pull request. The check sits in `lint` so that
  `gate` blocks the merge without a ruleset change. It prints only the
  commit and the field name on a match, so the log does not reveal what
  matched, and it receives the secret through `env:`, so shell syntax in
  the pattern cannot run.
- `test` used cargo-nextest, which reports every failing test in one run,
  and ran doctests with `cargo test --doc`, because nextest does not run
  doctests.
- Each crumbkit feature build that compiles code no other build compiles
  got its own clippy or test step in an existing job, because a single
  `--all-features` run misses a warning that only one build shows, and a
  new job would repeat the setup.
- A scheduled workflow ran the slow lane: the advisories check, the CI
  time limit, the docs.rs build on the nightly toolchain, the benchmarks
  and the Windows build and tests. The benchmarks and Windows skip a night
  when `main` has not changed since their last pass, because nothing they
  measure changed. A failure comments on one open issue, so every failure
  stays in one place.
- The macOS tests ran on each push to `main` and stayed out of pull
  requests, so a pull request never waits for a free macOS runner.
- Only runs on `main` saved a rust-cache, and pull requests restore it.
  Scheduled and release jobs used no rust-cache, because their cache keys
  would not help pull requests and a poisoned cache would reach the
  published crates.
- The workflows followed CI-1, because a broad token or a movable tag lets
  a compromised action change the repository or run unreviewed code.

## Consequences

Windows and macOS failures show after a merge, on the commit or the
scheduled issue, and do not block a pull request. `docs/project/ci.md`
lists the current jobs and steps.

Considered: two jobs on the single-CPU `ubuntu-slim` runner, one sequential job, a ruleset that requires branches to be up to date, no run on pushes to `main`, one job per check, a separate job for each crumbkit feature, a separate MSRV job, nextest partitions across jobs, the arm64 `ubuntu-24.04-arm` runner, larger runners, `cargo test` for every test, sccache, code coverage, macOS tests on every pull request, weekly or nightly macOS tests, a rust-cache key shared by all jobs, rust-cache saves from pull requests, one issue per failed run, the docs.rs build in `lint`, benchmarks every night whether or not `main` changed, Windows tests in the pull-request lane, Windows tests on each push to `main`, rust-cache on the Windows job, a Windows test step inside the `bench` job, a separate job for the commit check, a commit check that also runs on pushes to `main`.
