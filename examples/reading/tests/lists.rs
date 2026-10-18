//! End-to-end tests of the commands that reading adds for its lists. Each
//! test runs the built binary with `READING_DIR` set to the fixture lists
//! and compares its output with a golden file under `tests/golden/`
//! (ADR-0004, ADR-0005). `SNAPSHOTS=overwrite cargo test` rewrites the
//! golden files.

use std::io;
use std::path::Path;

use crumbkit::codes;
use crumbkit_test::{Examples, Failing, check_twin};
use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// Runs the built `reading` binary with `args`, `READING_DIR` set to `dir`, a
/// cleared environment and XDG directories in a temporary directory of its
/// own, and returns its output for assertions.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn reading(dir: &str, args: &[&str]) -> io::Result<OutputAssert> {
    reading_with(dir, args, &[])
}

/// Runs the built `reading` binary as [`reading`] does, with the variables
/// `env` added to its environment.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn reading_with(dir: &str, args: &[&str], env: &[(&str, &str)]) -> io::Result<OutputAssert> {
    let xdg = tempfile::tempdir()?;
    Ok(Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(args)
        .env_clear()
        .env("READING_DIR", dir)
        .env("XDG_CONFIG_HOME", xdg.path().join("config"))
        .env("XDG_DATA_HOME", xdg.path().join("data"))
        .env("XDG_STATE_HOME", xdg.path().join("state"))
        .env("XDG_CACHE_HOME", xdg.path().join("cache"))
        .envs(env.iter().copied())
        .assert())
}

#[test]
fn a_list_command_prints_the_entries_of_its_list_as_json() -> io::Result<()> {
    reading(LISTS, &["to-read", "list", "--json", "--limit", "2"])?
        .success()
        .stdout_eq(file!["golden/to-read-list-json-limit-2.json"]);
    Ok(())
}

#[test]
fn entries_list_with_a_limit_reads_only_the_page_so_its_footer_gives_no_count() -> io::Result<()> {
    reading(
        LISTS,
        &["entries", "list", "--list", "to-read", "--limit", "2"],
    )?
    .success()
    .stderr_eq("Showing 2 entries, and more match. --all shows every one.\n");
    Ok(())
}

#[test]
fn entries_list_with_json_and_a_limit_leaves_out_the_total_and_marks_the_page_truncated()
-> io::Result<()> {
    let output = reading(
        LISTS,
        &[
            "entries", "list", "--list", "to-read", "--limit", "2", "--json",
        ],
    )?
    .success()
    .get_output()
    .stdout
    .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(!stdout.contains(r#""total""#), "{stdout}");
    assert!(stdout.contains(r#""truncated":true"#), "{stdout}");
    Ok(())
}

#[test]
fn entries_list_with_json_and_all_holds_every_entry_and_no_total() -> io::Result<()> {
    let output = reading(
        LISTS,
        &["entries", "list", "--list", "to-read", "--all", "--json"],
    )?
    .success()
    .get_output()
    .stdout
    .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains(r#"{"position":5,"#), "{stdout}");
    assert!(stdout.contains(r#"}],"truncated":false}"#), "{stdout}");
    assert!(!stdout.contains(r#""total""#), "{stdout}");
    Ok(())
}

#[test]
fn entries_list_names_the_list_to_print() -> io::Result<()> {
    reading(LISTS, &["entries", "list", "--list", "finished"])?
        .success()
        .stdout_eq(file!["golden/entries-list-finished.txt"]);
    Ok(())
}

#[test]
fn lists_list_prints_each_list_as_its_own_line_with_its_about_text() -> io::Result<()> {
    reading(LISTS, &["lists", "list"])?
        .success()
        .stdout_eq("finished: Books finished this year\nto-read: Books to read next\n");
    Ok(())
}

#[test]
fn lists_list_with_fields_prints_crumbkit_s_table() -> io::Result<()> {
    reading(LISTS, &["lists", "list", "--fields", "name"])?
        .success()
        .stdout_eq("NAME\nfinished\nto-read\n");
    Ok(())
}

#[test]
fn entries_list_of_a_list_that_does_not_exist_fails_with_a_next_step() -> io::Result<()> {
    reading(LISTS, &["entries", "list", "--list", "unread"])?
        .code(1)
        .stdout_eq("")
        .stderr_eq(file!["golden/entries-list-unread.stderr"]);
    Ok(())
}

#[test]
fn a_directory_without_lists_adds_no_command() -> io::Result<()> {
    let empty = tempfile::tempdir()?;
    let dir = empty.path().to_string_lossy().into_owned();
    let env = [("READING_DIR", dir.as_str())];
    let examples = Examples::new(env!("CARGO_BIN_EXE_reading"), "").env(&env);

    // `commands list` differs by one word and runs in the same directory, so
    // the missing list is the reason for the refusal (TEST-1). The check
    // reads each line's command from the tree of the empty directory, where
    // `to-read` names no command.
    let refused = check_twin(
        &reading::command(&reading::lists_in(empty.path())),
        &reading::GUIDE,
        &examples,
        &Failing::new(&codes::CLI_USAGE, &["to-read", "list"]),
        &["commands", "list"],
    );
    OutputAssert::new(refused)
        .code(2)
        .stdout_eq("")
        .stderr_eq(file!["golden/to-read-without-lists.stderr"]);
    Ok(())
}

#[test]
fn entries_stream_of_a_list_that_does_not_exist_prints_the_header_and_then_the_failure()
-> io::Result<()> {
    reading(LISTS, &["entries", "stream", "--list", "unread"])?
        .code(1)
        .stdout_eq("POSITION  TITLE\n")
        .stderr_eq(file!["golden/entries-list-unread.stderr"]);
    Ok(())
}

#[test]
fn entries_stream_with_json_prints_a_failure_of_a_missing_list_as_its_only_line() -> io::Result<()>
{
    let output = reading(LISTS, &["entries", "stream", "--list", "unread", "--json"])?
        .code(1)
        .stderr_eq("")
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert_eq!(stdout.lines().count(), 1);
    assert!(
        stdout
            .starts_with(r#"{"schema":"reading/failure/v1","code":"reading::entries::not_found""#),
        "{stdout}"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn edit_opens_the_list_in_the_editor_that_visual_names_and_returns_its_exit_code() -> io::Result<()>
{
    use std::fs;
    use std::os::unix::fs::PermissionsExt as _;

    let scratch = tempfile::tempdir()?;
    let opened = scratch.path().join("opened.txt");
    let editor = scratch.path().join("editor");
    // The editor writes its arguments to `opened.txt`, one on each line,
    // and exits with 3.
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexit 3\n",
        opened.display()
    );
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the editor script that reading runs"
    )]
    fs::write(&editor, script)?;
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755))?;
    let visual = format!("{} --wait", editor.display());

    reading_with(LISTS, &["to-read", "edit"], &[("VISUAL", &visual)])?
        .code(3)
        .stdout_eq("")
        .stderr_eq("");
    let expected = format!("--wait\n{LISTS}/to-read.txt\n");
    assert_eq!(fs::read_to_string(opened)?, expected);
    Ok(())
}

/// Returns a temporary directory that holds a copy of the fixture lists, so
/// a test can write to a list without changing the fixtures.
///
/// # Errors
///
/// Returns the I/O error of a directory or a file that cannot be created.
fn copy_of_lists() -> io::Result<tempfile::TempDir> {
    let copy = tempfile::tempdir()?;
    for name in ["finished.txt", "to-read.txt"] {
        std::fs::copy(Path::new(LISTS).join(name), copy.path().join(name))?;
    }
    Ok(copy)
}

/// Returns the text of the to-read list in `dir`.
///
/// # Errors
///
/// Returns the I/O error of a file that cannot be read.
fn to_read(dir: &tempfile::TempDir) -> io::Result<String> {
    std::fs::read_to_string(dir.path().join("to-read.txt"))
}

/// The text of the to-read fixture list.
const TO_READ: &str = "# Books to read next\nNorth and South\nWuthering Heights\nThe Moonstone\nDracula\nLittle Women\n";

#[test]
fn entries_set_replaces_the_entry_when_the_expected_title_matches() -> io::Result<()> {
    let copy = copy_of_lists()?;
    let dir = copy.path().to_string_lossy().into_owned();

    reading(
        &dir,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "2",
            "--expect",
            "Wuthering Heights",
            "--yes",
            "Mary Barton",
        ],
    )?
    .success()
    .stdout_eq("POSITION  TITLE\n2         Mary Barton\n");
    assert_eq!(
        to_read(&copy)?,
        TO_READ.replace("Wuthering Heights", "Mary Barton")
    );
    Ok(())
}

#[test]
fn entries_set_with_expect_none_appends_an_entry_after_the_last() -> io::Result<()> {
    let copy = copy_of_lists()?;
    let dir = copy.path().to_string_lossy().into_owned();

    reading(
        &dir,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "6",
            "--expect",
            "none",
            "--yes",
            "Emma",
        ],
    )?
    .success();
    assert_eq!(to_read(&copy)?, format!("{TO_READ}Emma\n"));
    Ok(())
}

#[test]
fn entries_set_fails_with_a_conflict_and_changes_nothing_when_the_title_differs() -> io::Result<()>
{
    let copy = copy_of_lists()?;
    let dir = copy.path().to_string_lossy().into_owned();

    reading(
        &dir,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "2",
            "--expect",
            "Emma",
            "--yes",
            "Mary Barton",
        ],
    )?
    .code(1)
    .stdout_eq("")
    .stderr_eq(file!["golden/entries-set-conflict.stderr"]);
    assert_eq!(to_read(&copy)?, TO_READ);
    Ok(())
}

#[test]
fn entries_set_with_expect_none_fails_with_a_conflict_when_the_entry_exists() -> io::Result<()> {
    let copy = copy_of_lists()?;
    let dir = copy.path().to_string_lossy().into_owned();

    reading(
        &dir,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "5",
            "--expect",
            "none",
            "--yes",
            "Emma",
        ],
    )?
    .code(1)
    .stdout_eq("");
    assert_eq!(to_read(&copy)?, TO_READ);
    Ok(())
}

#[test]
fn entries_set_on_a_dry_run_prints_the_entry_and_the_dry_run_line_on_standard_error_and_changes_nothing()
-> io::Result<()> {
    let copy = copy_of_lists()?;
    let dir = copy.path().to_string_lossy().into_owned();

    reading(
        &dir,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "1",
            "--dry-run",
            "Emma",
        ],
    )?
    .success()
    .stdout_eq("POSITION  TITLE\n1         Emma\n")
    .stderr_eq("This was a dry run, so nothing changed.\n");
    assert_eq!(to_read(&copy)?, TO_READ);
    Ok(())
}

#[test]
fn entries_set_refuses_an_empty_expect_and_admits_its_twin_with_a_title() {
    let examples = Examples::new(env!("CARGO_BIN_EXE_reading"), "").env(&[("READING_DIR", LISTS)]);

    // The twin differs from the refused command line in the value of
    // `--expect` alone, and it runs on a dry run, so the empty value is the
    // reason for the refusal (TEST-1).
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples,
        &Failing::new(
            &codes::CLI_USAGE,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "1",
                "--expect",
                "",
                "--dry-run",
                "Emma",
            ],
        ),
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "1",
            "--expect",
            "North and South",
            "--dry-run",
            "Emma",
        ],
    );
    OutputAssert::new(refused).code(2).stdout_eq("");
}

#[test]
fn entries_set_refuses_a_title_with_a_line_break_and_admits_its_twin_without_one() {
    let examples = Examples::new(env!("CARGO_BIN_EXE_reading"), "").env(&[("READING_DIR", LISTS)]);

    // The twin differs from the refused command line in the title alone,
    // so the line break is the reason for the refusal (TEST-1).
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples,
        &Failing::new(
            &codes::CLI_USAGE,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "1",
                "--dry-run",
                "Emma\nWoodhouse",
            ],
        ),
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "1",
            "--dry-run",
            "Emma",
        ],
    );
    OutputAssert::new(refused).code(2).stdout_eq("");
}

#[test]
fn entries_set_fails_when_the_position_is_more_than_one_past_the_last_entry() -> io::Result<()> {
    reading(
        LISTS,
        &[
            "entries",
            "set",
            "--list",
            "to-read",
            "--position",
            "7",
            "--dry-run",
            "Emma",
        ],
    )?
    .code(1)
    .stdout_eq("")
    .stderr_eq(file!["golden/entries-set-out-of-range.stderr"]);
    Ok(())
}

/// Returns a temporary directory that holds the list `empty`, whose file has
/// a comment and a blank line and no entries.
///
/// # Errors
///
/// Returns the I/O error of a directory or a file that cannot be created.
fn empty_list() -> io::Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper writes the list file that the empty-list tests read"
    )]
    std::fs::write(dir.path().join("empty.txt"), "# Nothing yet\n\n")?;
    Ok(dir)
}

#[test]
fn entries_list_of_a_list_with_no_entries_prints_the_header_and_the_list_s_empty_note_on_standard_error()
-> io::Result<()> {
    let dir = empty_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["entries", "list", "--list", "empty"])?
        .success()
        .stdout_eq("POSITION  TITLE\n")
        .stderr_eq("note: the list has no entries\n");
    Ok(())
}

#[test]
fn entries_list_with_json_of_a_list_with_no_entries_prints_no_rows_and_no_total() -> io::Result<()>
{
    let dir = empty_list()?;
    let dir = dir.path().to_string_lossy();

    let output = reading(&dir, &["entries", "list", "--list", "empty", "--json"])?
        .success()
        .stderr_eq("")
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains(r#""data":[]"#), "{stdout}");
    assert!(!stdout.contains(r#""total""#), "{stdout}");
    Ok(())
}

#[test]
fn entries_stream_of_a_list_with_no_entries_prints_the_header_and_the_list_s_empty_note_on_standard_error()
-> io::Result<()> {
    let dir = empty_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["entries", "stream", "--list", "empty"])?
        .success()
        .stdout_eq("POSITION  TITLE\n")
        .stderr_eq("note: the list has no entries\n");
    Ok(())
}

#[test]
fn entries_stream_with_json_of_a_list_with_no_entries_prints_the_note_on_standard_error()
-> io::Result<()> {
    let dir = empty_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["entries", "stream", "--list", "empty", "--json"])?
        .success()
        .stdout_eq("")
        .stderr_eq("note: the list has no entries\n");
    Ok(())
}
