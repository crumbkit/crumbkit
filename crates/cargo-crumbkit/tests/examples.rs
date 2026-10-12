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
/// `RUSTUP_HOME` find cargo's registry and the toolchain,
/// `RUSTUP_TOOLCHAIN` names the toolchain that runs the test so rustup does
/// not look for another, and `CARGO_TARGET_DIR` keeps the build in the
/// test's target directory.
///
/// `CARGO_INCREMENTAL`, `CARGO_PROFILE_DEV_DEBUG` and `RUSTFLAGS` change how
/// cargo builds. weigh's cargo gets the values that built the test, so it
/// finds shelf already built and leaves the shared binary in place. A build
/// with other values rebuilds shelf, which can take longer than an example
/// may run, and on Windows it fails while another test runs the binary,
/// because Windows cannot replace a running executable.
const PASSED: [&str; 10] = [
    "CARGO",
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "CARGO_TARGET_DIR",
    "CARGO_INCREMENTAL",
    "CARGO_PROFILE_DEV_DEBUG",
    "RUSTFLAGS",
];

/// The variables that Windows programs need, which the test also passes on
/// there: `SystemRoot` lets the network stack and system libraries load,
/// `USERPROFILE` is the home directory of rustup and cargo, `TEMP` and `TMP`
/// hold temporary files, and `ProgramData`, `ProgramFiles` and
/// `ProgramFiles(x86)` let rustc find the MSVC linker for build scripts.
#[cfg(windows)]
const PLATFORM_PASSED: [&str; 7] = [
    "SystemRoot",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "ProgramData",
    "ProgramFiles",
    "ProgramFiles(x86)",
];

/// Unix programs need no variables beyond [`PASSED`].
#[cfg(not(windows))]
const PLATFORM_PASSED: [&str; 0] = [];

/// `weigh` passes cargo's standard error through to its own, and an example
/// must write nothing there, so the test turns off cargo's progress lines.
/// cargo still prints its errors with `CARGO_TERM_QUIET` set.
const QUIET: (&str, &str) = ("CARGO_TERM_QUIET", "true");

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    let passed: Vec<(&str, String)> = PASSED
        .iter()
        .chain(&PLATFORM_PASSED)
        .filter_map(|&name| {
            #[expect(
                clippy::disallowed_methods,
                reason = "the test owns its environment reads, which it passes on to weigh's cargo"
            )]
            let value = std::env::var(name).ok()?;
            Some((name, value))
        })
        .collect();
    let mut env: Vec<(&str, &str)> = passed
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    env.push(QUIET);
    let examples = Examples::new(
        env!("CARGO_BIN_EXE_cargo-crumbkit"),
        "tests/golden/examples",
    )
    .env(&env);

    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &examples);
}
