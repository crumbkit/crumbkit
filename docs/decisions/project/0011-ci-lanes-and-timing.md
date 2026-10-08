# ADR-0011. CI lanes

- Pull requests run a fast lane of parallel jobs behind one required `gate` job, with a median wall time of at most 5 minutes.
- A nightly slow lane runs fuzzing, advisories, macOS, benchmarks and dependency checks, and reports failures on one issue.
- Workflows start with `permissions: {}` and pin actions to commit SHAs.

Considered: one sequential job, sccache, code coverage.
