//! End-to-end tests of `skill draft`, which build shelf and draft its skill
//! file from shelf's commands and a fixture directory of exported usage.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

/// The target directory of the cargo builds in these tests, which the
/// other end-to-end tests share. `skill draft` builds shelf with shelf's
/// own features, so a build in the shared target directory would replace
/// the shelf binary that other tests run.
const TARGET_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/weigh");

/// Exported usage of shelf from two machines, which repeats one run and
/// holds runs of `books lend`, a command that shelf does not have.
const USAGE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-usage-export"
);

/// The skill file that a draft into a missing file writes from [`USAGE`].
const DRAFTED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/shelf-skill-drafted.md"
);

/// Runs `cargo crumbkit skill <args>` and returns its output.
fn skill<A: AsRef<OsStr>>(args: impl IntoIterator<Item = A>) -> std::io::Result<Output> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "skill"])
        .args(args)
        .env("CARGO_TERM_QUIET", "true")
        .env("CARGO_TARGET_DIR", TARGET_DIR)
        .output()
}

/// Runs `cargo crumbkit skill draft --bin shelf --skill <file>` with `args`
/// after it.
fn draft(file: &Path, args: &[&str]) -> std::io::Result<Output> {
    let words = ["draft", "--bin", "shelf", "--skill"].map(OsStr::new);
    skill(
        words
            .into_iter()
            .chain([file.as_os_str()])
            .chain(args.iter().map(OsStr::new)),
    )
}

#[test]
fn a_draft_into_a_missing_file_writes_the_skeleton_with_every_section() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("skill/SKILL.md");

    let output = draft(&file, &["--usage", USAGE]).unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert_eq!(
        stdout,
        format!(
            "Drafted {} for shelf from 18 runs, and ignored 2 that name a command shelf no longer has or another tool.\n\
             SECTION      STATUS\n\
             description  written\n\
             jobs         written\n\
             usage        written\n",
            file.display()
        )
    );
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        std::fs::read_to_string(DRAFTED).unwrap()
    );
}

#[test]
fn every_command_line_of_the_drafted_skill_passes_skill_check() {
    let output = skill(["check", "--bin", "shelf", "--skill", DRAFTED]).unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert_eq!(
        stdout,
        format!("Checked 11 shelf command lines in {DRAFTED}, and 0 failed.\n")
    );
}

#[test]
fn a_draft_rewrites_only_the_generated_sections_and_keeps_the_hand_written_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("SKILL.md");
    let drafted = std::fs::read_to_string(DRAFTED).unwrap();
    let hand_written = "Use shelf to read books.\r\nIt never  reflows   this.\n";
    let stale = drafted
        .replace("<!-- Write here", &format!("{hand_written}<!-- Write here"))
        .replace("| Review how shelf is used |", "| Old heading |");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the skill file that the draft rewrites"
    )]
    std::fs::write(&file, &stale).unwrap();

    let output = draft(&file, &[]).unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        drafted.replace("<!-- Write here", &format!("{hand_written}<!-- Write here"))
    );
}

#[test]
fn a_check_of_a_stale_file_names_the_section_writes_nothing_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("SKILL.md");
    let stale = std::fs::read_to_string(DRAFTED)
        .unwrap()
        .replace("- `shelf books summarize`", "- `shelf books lend`");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the skill file that the check reads"
    )]
    std::fs::write(&file, &stale).unwrap();

    let output = draft(&file, &["--usage", USAGE, "--check"]).unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("SECTION      STATUS\nusage        would-change\n"),
        "{stdout}"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), stale);
}
