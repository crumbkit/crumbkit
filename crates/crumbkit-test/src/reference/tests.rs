//! Tests for the command reference check.
//!
//! Each test writes a shell script named `tool` that prints a fixed
//! `commands list` document, so the tests need no binary of their own. They
//! run only on Unix.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// A `commands list --all --json --full` document with one command.
const DOCUMENT: &str = r#"{"schema":"crumbkit/commands-list/v1","data":[{"path":"tool","about":"Browse a catalogue","long_about":null,"arguments":[],"examples":[],"columns":[],"form":null,"exits":[],"effects":null}],"total":1}"#;

/// The reference of [`DOCUMENT`].
const REFERENCE: &str = "# Commands of `tool`

crumbkit-markdown writes this reference from the help of `tool`. Edit the help in the tool's code, and not this file.

## `tool`

Browse a catalogue
";

/// Writes the executable script `tool` in `dir`, whose body is `script`,
/// and returns its path.
fn tool(dir: &TempDir, script: &str) -> PathBuf {
    dir.write("tool", &format!("#!/bin/sh\n{script}\n"));
    let path = dir.path().join("tool");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Returns a tool in `dir` that prints [`DOCUMENT`] when it is asked for
/// its commands list.
fn listing_tool(dir: &TempDir) -> PathBuf {
    tool(
        dir,
        &format!(
            "[ \"$*\" = 'commands list --all --json --full' ] || exit 2\nprintf '%s\\n' '{DOCUMENT}'"
        ),
    )
}

/// Runs the check of the tool `binary` against `file` with `action`.
fn run(binary: &Path, file: &Path, action: Action) {
    check(
        binary.to_str().unwrap(),
        file,
        &Assert::new().action(action),
    );
}

#[test]
fn overwrite_writes_the_reference_to_the_file_and_its_directory() {
    let dir = TempDir::new("reference-overwrite").unwrap();
    let file = dir.path().join("docs").join("commands.md");

    run(&listing_tool(&dir), &file, Action::Overwrite);

    assert_eq!(fs::read_to_string(&file).unwrap(), REFERENCE);
}

#[test]
fn a_file_that_holds_the_tool_s_reference_passes() {
    let dir = TempDir::new("reference-pass").unwrap();
    dir.write("commands.md", REFERENCE);

    run(
        &listing_tool(&dir),
        &dir.path().join("commands.md"),
        Action::Verify,
    );
}

#[test]
fn a_missing_file_fails_and_says_that_snapshots_overwrite_writes_it() {
    let dir = TempDir::new("reference-missing").unwrap();
    let file = dir.path().join("commands.md");

    let message = panic_message(|| run(&listing_tool(&dir), &file, Action::Verify)).unwrap();

    assert_eq!(
        message,
        format!(
            "the command tree breaks crumbkit's command reference rules:\n\
             tool commands list --all --json --full: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            file.display()
        )
    );
}

#[test]
fn a_file_that_differs_from_the_reference_fails_and_shows_both_texts() {
    let dir = TempDir::new("reference-differs").unwrap();
    dir.write("commands.md", "# Commands of `shelf`\n");
    let file = dir.path().join("commands.md");

    let message = panic_message(|| run(&listing_tool(&dir), &file, Action::Verify)).unwrap();

    let heading = format!(
        "the command tree breaks crumbkit's command reference rules:\n\
         tool commands list --all --json --full: the reference differs from the golden file `{}`:\n",
        file.display()
    );
    assert!(message.starts_with(&heading), "{message}");
    assert!(message.contains("shelf"), "{message}");
    assert!(message.contains("Browse a catalogue"), "{message}");
}

#[test]
fn a_tool_whose_commands_list_prints_another_document_fails_with_the_reason() {
    let dir = TempDir::new("reference-other").unwrap();
    let binary = tool(&dir, r#"printf '%s\n' '{"schema":"tool/books/v1"}'"#);
    let file = dir.path().join("commands.md");

    let message = panic_message(|| run(&binary, &file, Action::Verify)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's command reference rules:\n\
         tool commands list --all --json --full: the reference cannot be written: \
         the input is a tool/books/v1 document, and the reference needs a crumbkit/commands-list/v1 document"
    );
}

#[test]
fn a_tool_whose_commands_list_fails_reports_its_exit_status_and_standard_error() {
    let dir = TempDir::new("reference-fails").unwrap();
    let binary = tool(&dir, "echo 'unknown command' >&2\nexit 2");
    let file = dir.path().join("commands.md");

    let message = panic_message(|| run(&binary, &file, Action::Verify)).unwrap();

    assert!(
        message.starts_with(
            "the command tree breaks crumbkit's command reference rules:\n\
             tool commands list --all --json --full: the run must exit with code 0, and it ends with "
        ),
        "{message}"
    );
    assert!(
        message.ends_with(" after writing this to standard error:\nunknown command\n"),
        "{message}"
    );
}

#[test]
fn a_binary_that_cannot_start_fails_and_names_the_binary() {
    let dir = TempDir::new("reference-no-binary").unwrap();
    let binary = dir.path().join("tool");
    let file = dir.path().join("commands.md");

    let message = panic_message(|| run(&binary, &file, Action::Verify)).unwrap();

    assert!(
        message.starts_with(&format!(
            "the command tree breaks crumbkit's command reference rules:\n\
             tool commands list --all --json --full: the binary `{}` cannot start: ",
            binary.display()
        )),
        "{message}"
    );
}
