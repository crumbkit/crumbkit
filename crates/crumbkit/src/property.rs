//! The configuration that every property test in crumbkit runs with
//! (ADR-0006).

use proptest::test_runner::{Config, RngSeed};

/// The seed of every property test's random number generator. Any fixed
/// value makes each run generate the same cases, so a failure in CI repeats
/// on every machine. A different value generates a different set of cases.
const SEED: u64 = 1;

/// Returns proptest's default configuration with a fixed seed and without
/// failure persistence. Every `proptest!` block starts with
/// `#![proptest_config(crate::property::config())]`, and `cargo xtask check`
/// fails on a block that does not.
///
/// By default proptest writes the seed of each failing case to a
/// `proptest-regressions` file and replays it first on the next run. crumbkit
/// turns that off, because a fix adds a unit test that holds the failing input
/// instead (ADR-0006).
///
/// Each test runs proptest's default of 256 cases. proptest reads its
/// `PROPTEST_` environment variables over this configuration, so
/// `PROPTEST_CASES=10000` runs more cases and `PROPTEST_RNG_SEED=7` another
/// seed for one run. CI sets neither.
pub(crate) fn config() -> Config {
    Config {
        rng_seed: RngSeed::Fixed(SEED),
        failure_persistence: None,
        ..Config::default()
    }
}
