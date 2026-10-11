//! End-to-end test of a failed build: `weigh` passes cargo's standard error
//! through to its own, so a person sees why the build failed.

use std::process::Command;

#[test]
fn a_failed_build_shows_cargo_s_own_error_on_standard_error() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "weigh", "--bin", "no-such-binary"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("no bin target named `no-such-binary`"),
        "cargo's error is missing from standard error:\n{stderr}"
    );
    assert!(
        stderr.contains("crumbkit::weigh::build_failed"),
        "weigh's failure is missing from standard error:\n{stderr}"
    );
}
