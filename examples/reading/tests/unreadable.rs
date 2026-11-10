//! End-to-end tests of the failure for a list whose file cannot be read as
//! UTF-8 text, whose alternative is a command that a person runs in a
//! terminal.

use std::io;

use snapbox::cmd::{Command, OutputAssert};

/// Runs the built `reading` binary with `args` in a cleared environment,
/// with `READING_DIR` set to a directory that holds the list `name`, whose
/// second line is not UTF-8, and returns its output for assertions.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory or file that cannot be
/// created.
#[expect(
    clippy::disallowed_methods,
    reason = "the test writes the fixture list into its own temporary directory"
)]
fn reading(name: &str, args: &[&str]) -> io::Result<OutputAssert> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join(format!("{name}.txt")), b"Emma\n\xff\n")?;
    let xdg = dir.path().join("xdg");
    Ok(Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(args)
        .env_clear()
        .env("READING_DIR", dir.path())
        .env("XDG_CONFIG_HOME", xdg.join("config"))
        .env("XDG_DATA_HOME", xdg.join("data"))
        .env("XDG_STATE_HOME", xdg.join("state"))
        .env("XDG_CACHE_HOME", xdg.join("cache"))
        .assert())
}

#[test]
fn a_list_that_is_not_utf8_offers_its_edit_command_in_a_terminal() -> io::Result<()> {
    let output = reading("bad", &["entries", "list", "--list", "bad"])?
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    assert!(
        stderr.contains("\n  help:  run `reading bad edit` in a terminal\n"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_list_that_is_not_utf8_offers_a_terminal_step_in_json() -> io::Result<()> {
    let output = reading("bad", &["entries", "list", "--list", "bad", "--json"])?
        .code(1)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains(r#""alternatives":[{"kind":"terminal","call":"reading bad edit"}]"#),
        "{stdout}"
    );
    Ok(())
}

#[test]
fn a_list_whose_name_is_reserved_offers_no_edit_command() -> io::Result<()> {
    let output = reading("help", &["entries", "list", "--list", "help", "--json"])?
        .code(1)
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(!stdout.contains("alternatives"), "{stdout}");
    Ok(())
}
