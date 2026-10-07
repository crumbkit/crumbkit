# 0002. Public API and semver

- Each release pull request reviews the Rust API Guidelines checklist and runs cargo-semver-checks.
- Configuration types have private fields and `const fn` builders. Enums that may grow are `#[non_exhaustive]`, and adopter-implemented traits gain only methods with defaults.
- Handlers take an arguments struct deriving `clap::Args`, `Serialize` and `JsonSchema`, and never see `ArgMatches`.
- Failure codes are never reused, and exit codes 0, 1, 2 and 130 are fixed.

Considered: public-field structs, a feature-gated schema bound.
