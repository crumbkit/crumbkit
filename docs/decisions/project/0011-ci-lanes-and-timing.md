# ADR-0011. CI lanes

- Pull requests run a fast lane with a median wall time of at most 5 minutes. While the repository is private, the fast lane has two parallel work jobs, `lint` and `test`, on the single-CPU `ubuntu-slim` runner, because GitHub bills each job's minutes rounded up to a whole minute, every extra job repeats its setup time, and a slim minute costs a third of an `ubuntu-latest` minute.
- The fast lane adds one required `gate` job when the repository becomes public and branch protection can require it. Until then `lint` and `test` report on their own.
- `test` runs the tests with cargo-nextest, whose `ci` profile reports every failing test in one run, and runs the doctests with `cargo test --doc`, because nextest does not run doctests. It also checks the MSRV with `cargo +1.89.0 check`.
- A nightly slow lane runs fuzzing, advisories, benchmarks and dependency checks, and a weekly run tests on macOS, because a macOS minute costs ten times a Linux minute. Both report failures on one issue.
- Workflows start with `permissions: {}` and pin actions to commit SHAs, and zizmor lints the workflows in the `lint` job.

Considered: one sequential job, one job per check, `ubuntu-latest` runners, `cargo test` for every test, sccache, code coverage.
