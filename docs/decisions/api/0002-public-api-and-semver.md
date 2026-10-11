# ADR-0002. Public API and semver

- Each release pull request will review the Rust API Guidelines checklist and run cargo-semver-checks once a release workflow exists.
- crumbkit re-exports every public item of crumbkit-core at the path the item had in crumbkit, so crumbkit-core's public API is part of crumbkit's (ADR-0032). `Call::help`, `Call::word` and `Call::target` are public and documented like every other item, because crumbkit builds and renders calls from outside crumbkit-core.
- Configuration types have private fields and `const fn` builders. Enums that may grow are `#[non_exhaustive]`, and adopter-implemented traits gain only methods with defaults.
- Handlers take an arguments struct deriving `clap::Args`, `Serialize` and `JsonSchema`, and never see `ArgMatches`.
- Failure codes are never reused, and exit codes 0, 1 and 2 are fixed. A shell reports 130 when SIGINT ends the process and 143 when SIGTERM does, and the usage log's signal handling leaves those codes in place.

Considered: public-field structs, a feature-gated schema bound, `Call` methods hidden from the docs.
