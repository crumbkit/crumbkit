//! End-to-end tests of `tool-skill check`, which build shelf and check a skill
//! file and a skill folder whose command lines fail against shelf's commands,
//! and build cargo crumbkit and check its own skill.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

/// The target directory of the cargo builds of `tool-skill check` in these
/// tests, which weigh's and docs' end-to-end tests share. `tool-skill check`
/// builds shelf with shelf's own features, so a build in the shared target
/// directory would replace the shelf binary that other tests run.
const TARGET_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/weigh");

/// A skill file with one passing command line in prose, three failing ones
/// in a fenced block and one more passing one in prose.
const FAILING: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-skill-failing.md"
);

/// A skill folder whose `SKILL.md` passes, whose `REVIEW.md` and
/// `lending/SKILL.md` each hold a failing command line, and whose `notes.txt`
/// holds one that the check never reads.
const FOLDER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-skill-folder"
);

/// Runs `cargo crumbkit tool-skill check --bin shelf --skill <FAILING>` with
/// `args` after it.
fn check(args: &[&str]) -> std::io::Result<Output> {
    check_path(FAILING, args)
}

/// Runs `cargo crumbkit tool-skill check --bin shelf --skill <skill>` with
/// `args` after it.
fn check_path(skill: &str, args: &[&str]) -> std::io::Result<Output> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args([
            "crumbkit",
            "tool-skill",
            "check",
            "--bin",
            "shelf",
            "--skill",
            skill,
        ])
        .args(args)
        .env("CARGO_TERM_QUIET", "true")
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
}

#[test]
fn a_skill_with_failing_command_lines_prints_each_failure_and_exits_1() {
    let output = check(&[]).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert_eq!(
        stdout,
        format!(
            "Checked 5 shelf command lines in {FAILING}, and 3 failed.\n\
             {FAILING}:6: shelf books lsit: `lsit` is not a command of `shelf books`\n\
             {FAILING}:7: shelf books list --limit: `--limit` needs a value\n\
             {FAILING}:8: shelf books list --author Austen: `shelf books list` has no flag `--author`\n"
        )
    );
}

#[test]
fn json_holds_every_command_line_with_the_failing_ones_first() {
    let output = check(&["--json"]).unwrap();
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document["schema"], "crumbkit/tool-skill-check/v1");
    let rows: Vec<(u64, Option<&str>)> = document["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["line"].as_u64().unwrap(), row["reason"].as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (6, Some("`lsit` is not a command of `shelf books`")),
            (7, Some("`--limit` needs a value")),
            (8, Some("`shelf books list` has no flag `--author`")),
            (3, None),
            (11, None),
        ]
    );
    assert_eq!(document["summary"]["failed"], 3);
}

#[test]
fn a_skill_folder_prints_the_failures_of_every_markdown_file_in_it_and_exits_1() {
    let output = check_path(FOLDER, &[]).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let review = Path::new(FOLDER).join("REVIEW.md");
    let lending = Path::new(FOLDER).join("lending").join("SKILL.md");

    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert_eq!(
        stdout,
        format!(
            "Checked 4 shelf command lines in {FOLDER}, and 2 failed.\n\
             {}:5: shelf books list --author Austen: `shelf books list` has no flag `--author`\n\
             {}:6: shelf books sumarize: `sumarize` is not a command of `shelf books`\n",
            review.display(),
            lending.display()
        )
    );
}

#[test]
fn every_command_line_of_cargo_crumbkit_s_own_skill_passes_tool_skill_check() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "tool-skill", "check", "--bin", "cargo-crumbkit"])
        .args(["--long"])
        .env("CARGO_TERM_QUIET", "true")
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(
        stdout.contains("cargo crumbkit tool-skill check --bin <NAME>"),
        "{stdout}"
    );
}
