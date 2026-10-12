//! The example test of cargo-crumbkit. It runs every example in the guide
//! against the built binary and compares each run's standard output with its
//! golden file under `tests/golden/examples/` (ADR-0005).
//! `SNAPSHOTS=overwrite cargo test` writes, rewrites and deletes those files.
//!
//! The examples of `cargo crumbkit weigh` build and weigh shelf, so this test
//! is also weigh's end-to-end test, and its golden files pin weigh's text and
//! JSON output for shelf. They change whenever shelf's help pages or example
//! golden files change, and `SNAPSHOTS=overwrite` rewrites them.
//!
//! The example of `cargo crumbkit docs` would write shelf's reference into
//! the workspace, so cargo-crumbkit opts out of running it, and
//! `tests/docs.rs` runs `docs` with a file in a temporary directory.

use cargo_crumbkit::{ALLOW, Cli, GUIDE};
use clap::CommandFactory as _;
use crumbkit_test::Examples;

/// The variables that weigh's cargo build needs, which the test passes on
/// from its own environment when they are set: `CARGO` names the cargo that
/// runs the test, `PATH` finds `rustc`, `HOME`, `CARGO_HOME` and
/// `RUSTUP_HOME` find cargo's registry and the toolchain, and
/// `RUSTUP_TOOLCHAIN` names the toolchain that runs the test so rustup does
/// not look for another.
const PASSED: [&str; 6] = [
    "CARGO",
    "PATH",
    "HOME",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
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

/// The target directory of weigh's cargo build, which only this test uses.
///
/// weigh builds shelf with shelf's own features, while the test build
/// unifies the features of every package and its dev-dependencies, so a
/// build in the shared target directory would replace the shelf binary
/// that other tests run. Windows cannot replace a running executable, and
/// on any platform the other tests would run a binary that changed under
/// them.
const TARGET_DIR: (&str, &str) = (
    "CARGO_TARGET_DIR",
    concat!(env!("CARGO_TARGET_TMPDIR"), "/weigh-examples"),
);

/// `weigh` passes cargo's standard error through to its own, and an example
/// must write nothing there, so the test turns off cargo's progress lines.
/// cargo still prints its errors with `CARGO_TERM_QUIET` set.
const QUIET: (&str, &str) = ("CARGO_TERM_QUIET", "true");

#[test]
fn every_example_in_the_guide_runs_prints_its_golden_output_and_keeps_its_output_budget() {
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
    env.extend([TARGET_DIR, QUIET]);
    let built = build_shelf(&env).unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let examples = Examples::new(
        env!("CARGO_BIN_EXE_cargo-crumbkit"),
        "tests/golden/examples",
    )
    .env(&env)
    .allow(ALLOW);

    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &examples);
    crumbkit_test::check_output_budget(&Cli::command(), &GUIDE, &examples);
}

/// Builds shelf as weigh's cargo will, with the environment `env` alone, and
/// returns cargo's output, so the examples find shelf built and none of them
/// waits for a build that could take longer than an example may run.
fn build_shelf(env: &[(&str, &str)]) -> std::io::Result<std::process::Output> {
    let cargo = env
        .iter()
        .find(|(name, _)| *name == "CARGO")
        .map_or("cargo", |(_, value)| *value);
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds shelf before weigh's examples run"
    )]
    std::process::Command::new(cargo)
        .args(["build", "--workspace", "--bin", "shelf"])
        .env_clear()
        .envs(env.iter().copied())
        .output()
}
