//! The example test of cargo-crumbkit. It runs every example in the guide
//! against the built binary and compares each run's standard output with its
//! golden file under `tests/golden/examples/` (ADR-0005).
//! `SNAPSHOTS=overwrite cargo test` writes, rewrites and deletes those files.
//!
//! The examples of `cargo crumbkit weigh` build and weigh shelf, so this test
//! is also weigh's end-to-end test, and its golden files pin weigh's text and
//! JSON output for shelf. They change whenever shelf's help pages or example
//! golden files change, and `SNAPSHOTS=overwrite` rewrites them.

use cargo_crumbkit::{Cli, GUIDE};
use clap::CommandFactory as _;
use crumbkit_test::Examples;

/// The variables that weigh's cargo build needs, which the test passes on
/// from its own environment when they are set: `CARGO` names the cargo that
/// runs the test, `PATH` finds `rustc`, `HOME`, `CARGO_HOME` and
/// `RUSTUP_HOME` find cargo's registry and the toolchain, and
/// `CARGO_TARGET_DIR` keeps the build in the test's target directory.
const PASSED: [&str; 6] = [
    "CARGO",
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "CARGO_TARGET_DIR",
];

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    let passed: Vec<(&str, String)> = PASSED
        .iter()
        .filter_map(|&name| {
            #[expect(
                clippy::disallowed_methods,
                reason = "the test owns its environment reads, which it passes on to weigh's cargo"
            )]
            let value = std::env::var(name).ok()?;
            Some((name, value))
        })
        .collect();
    let env: Vec<(&str, &str)> = passed
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    let examples = Examples::new(
        env!("CARGO_BIN_EXE_cargo-crumbkit"),
        "tests/golden/examples",
    )
    .env(&env);

    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &examples);
}
