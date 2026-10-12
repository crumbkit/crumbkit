//! The end-to-end test of `docs`, which writes shelf's reference to a
//! temporary file and compares it with the reference that shelf checks in.

use std::fs;
use std::process::Command;

/// The target directory of the cargo builds of `docs` and `weigh` in these
/// tests. weigh's end-to-end test uses the same directory, and cargo's lock
/// on it makes the builds wait for each other. `docs` builds shelf with
/// shelf's own features, so a build in the shared target directory would
/// replace the shelf binary that other tests run.
const TARGET_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/weigh");

/// The reference that shelf checks in, which its conventions test keeps in
/// step with its help.
const SHELF_REFERENCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/shelf/docs/commands.md"
);

#[test]
fn docs_writes_the_reference_that_shelf_checks_in_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("reference").join("commands.md");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "docs", "--bin", "shelf", "--out"])
        .arg(&file)
        .env("CARGO_TERM_QUIET", "true")
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "Wrote the reference of 15 commands to {}.\n",
            file.display()
        )
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        fs::read_to_string(SHELF_REFERENCE).unwrap()
    );
}
