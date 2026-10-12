//! End-to-end tests of `weigh`: a failed build, whose cargo error `weigh`
//! passes through to its own standard error, and the usage figures of a
//! checked-in usage log of shelf.

use std::fs;
use std::process::Command;

use serde_json::Value;

/// The target directory of weigh's cargo builds in these tests, which only
/// these tests use. weigh builds shelf with shelf's own features, so a build
/// in the shared target directory would replace the shelf binary that other
/// tests run, and Windows cannot replace a running executable.
const TARGET_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/weigh");

#[test]
fn a_failed_build_shows_cargo_s_own_error_on_standard_error() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "weigh", "--bin", "no-such-binary"])
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("no bin target named `no-such-binary`"),
        "cargo's error is missing from standard error:\n{stderr}"
    );
    assert!(
        stderr.contains("crumbkit::tool::build_failed"),
        "weigh's failure is missing from standard error:\n{stderr}"
    );
}

/// The usage log of eleven runs of shelf on 1 October 2026: three help
/// pages, five runs of `books show` with two failures, and three runs of
/// `books list`, one of them refused. shelf's `self usage` JSON test reads it
/// too, so a change to it also changes that test's golden file.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-usage.jsonl"
);

#[test]
fn log_joins_the_figures_of_a_usage_log_to_each_command_and_leaves_the_log_unchanged() {
    let before = fs::read(FIXTURE).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "weigh", "--bin", "shelf", "--json"])
        .args(["--log", FIXTURE, "--since", "100y"])
        .env("CARGO_TERM_QUIET", "true")
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let figures = |path: &str| {
        let row = document["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["path"] == path)
            .unwrap();
        [&row["runs"], &row["help_before"], &row["p95_output"]].map(Value::as_u64)
    };

    assert_eq!(figures("shelf"), [Some(0), Some(0), None]);
    assert_eq!(figures("shelf books list"), [Some(3), Some(0), Some(512)]);
    assert_eq!(figures("shelf books show"), [Some(5), Some(3), Some(86)]);
    assert_eq!(figures("shelf commands list"), [Some(0), Some(0), None]);
    assert_eq!(fs::read(FIXTURE).unwrap(), before);
}
