# ADR-0002. Public API and semver

- Each release pull request that release-plz opens runs cargo-semver-checks against the last published version, so the checks start at 0.1.1, and its review goes through the Rust API Guidelines checklist (ADR-0012).
- crumbkit re-exports every public item of crumbkit-core at the path the item had in crumbkit, so crumbkit-core's public API is part of crumbkit's (ADR-0032). `Call::help`, `Call::word` and `Call::target` are public and documented like every other item, because crumbkit builds and renders calls from outside crumbkit-core.
- Configuration types have private fields and `const fn` builders. Enums that may grow are `#[non_exhaustive]`, and adopter-implemented traits gain only methods with defaults.
- Handlers take an arguments struct deriving `clap::Args`, `Serialize` and `JsonSchema`, and never see `ArgMatches`.
- Failure codes are never reused. crumbkit owns the exit codes 0, 1 and 2, whose meanings are fixed, and the tool owns 3 to 255, which crumbkit never returns. A shell reports 130 when SIGINT ends the process and 143 when SIGTERM does, and the usage log's signal handling leaves those codes in place.
- `parse` returns a [`Stop`](../../../crates/crumbkit/src/parse/stop.rs) in place of a bare `ExitCode` when it ends the run, so a tool can still ask whether the caller wanted JSON. Its `Output` sits in a `Box`, because `Output` holds a whole `clap::Command` and clippy's `result_large_err` flags a large error type, and the box costs one allocation only on a run that `parse` ends.

Considered: `parse` returning an `ExitCode` and a separate function that finds `--json`, a `Stop` with its own `is_json` and no `Output`, public-field structs, a feature-gated schema bound, `Call` methods hidden from the docs.
