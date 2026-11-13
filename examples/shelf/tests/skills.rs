//! End-to-end tests of shelf's `skills` commands. Each test runs the built
//! binary with `HOME` set to a temporary directory of its own, so no test
//! installs into the user's home directory (ADR-0192).

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use snapbox::cmd::{Command, OutputAssert};
use tempfile::TempDir;

/// Returns a temporary home with an empty `.agents/skills` directory.
///
/// # Errors
///
/// Returns the I/O error of a directory that cannot be created.
fn home() -> io::Result<TempDir> {
    let home = tempfile::tempdir()?;
    fs::create_dir(home.path().join(".agents"))?;
    fs::create_dir(home.path().join(".agents").join("skills"))?;
    Ok(home)
}

/// Runs the built `shelf` binary with `args`, a cleared environment, `HOME`
/// set to `home` and the XDG state directory under it, and returns its
/// output for assertions.
fn shelf(home: &Path, args: &[&str]) -> OutputAssert {
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .assert()
}

/// Returns the standard output of a run as a JSON value.
///
/// # Errors
///
/// Returns the error of standard output that is not JSON.
fn json(output: &OutputAssert) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&output.get_output().stdout)?)
}

#[test]
fn install_writes_the_skill_with_metadata_naming_shelf() {
    let home = home().unwrap();

    let output = shelf(home.path(), &["skills", "install", "--json"]).success();

    let document = json(&output).unwrap();
    assert_eq!(document["data"][0]["state"], "written");
    let text = fs::read_to_string(home.path().join(".agents/skills/shelf/SKILL.md")).unwrap();
    assert!(text.starts_with("---\nname: shelf\n"), "{text}");
    assert!(
        text.contains("metadata:\n  tool: \"shelf\"\n  version: \""),
        "{text}"
    );
}

#[test]
fn check_exits_with_1_and_names_a_missing_copy() {
    let home = home().unwrap();

    let output = shelf(home.path(), &["skills", "check"]).code(1);

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("missing"), "{stdout}");
    // The table writes a list cell as compact JSON, which doubles a Windows
    // separator, so the test searches for the path written as a JSON string.
    let dir = serde_json::to_string(&Path::new(".agents").join("skills")).unwrap();
    assert!(stdout.contains(dir.trim_matches('"')), "{stdout}");
}

#[test]
fn check_exits_with_0_and_prints_no_row_after_install() {
    let home = home().unwrap();
    shelf(home.path(), &["skills", "install"]).success();

    let output = shelf(home.path(), &["skills", "check"]).success();

    assert_eq!(output.get_output().stdout, [] as [u8; 0]);
}

#[test]
fn check_exits_with_1_when_the_copy_was_edited() {
    let home = home().unwrap();
    shelf(home.path(), &["skills", "install"]).success();
    let file = home.path().join(".agents/skills/shelf/SKILL.md");
    let edited = fs::read_to_string(&file).unwrap() + "An edit.\n";
    #[expect(
        clippy::disallowed_methods,
        reason = "the test edits the installed copy, as a person might"
    )]
    fs::write(&file, edited).unwrap();

    let output = shelf(home.path(), &["skills", "check", "--json"]).code(1);

    assert_eq!(json(&output).unwrap()["data"][0]["state"], "differs");
}

#[test]
fn print_prints_the_embedded_skill() {
    let home = home().unwrap();

    shelf(home.path(), &["skills", "print"])
        .success()
        .stdout_eq(include_str!("../skill/SKILL.md"));
}
