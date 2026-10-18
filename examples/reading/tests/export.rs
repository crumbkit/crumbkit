//! End-to-end tests of `reading entries export`, which prints the bytes of a
//! list's file. Each test runs the built binary with `READING_DIR` set to
//! the fixture lists or to a temporary directory.

use std::io;
use std::path::Path;

use crumbkit::codes;
use crumbkit_test::{Examples, Failing, check_twin};
use snapbox::cmd::{Command, OutputAssert};

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// The text of the to-read fixture list.
const TO_READ: &str = "# Books to read next\nNorth and South\nWuthering Heights\nThe Moonstone\nDracula\nLittle Women\n";

/// The bytes of a list whose second line is in Latin-1 and so is not
/// UTF-8: the byte `0xe9` is `é` in Latin-1, and UTF-8 reads it as the
/// start of a three-byte character that `r` does not continue.
const LATIN_1: &[u8] = b"# Books in Latin-1\nLes Mis\xe9rables\n";

/// Runs the built `reading` binary with `args`, `READING_DIR` set to `dir`, a
/// cleared environment and XDG directories in a temporary directory of its
/// own, and returns its output for assertions.
///
/// # Errors
///
/// Returns the I/O error of a temporary directory that cannot be created.
fn reading(dir: &str, args: &[&str]) -> io::Result<OutputAssert> {
    let xdg = tempfile::tempdir()?;
    Ok(Command::new(env!("CARGO_BIN_EXE_reading"))
        .args(args)
        .env_clear()
        .env("READING_DIR", dir)
        .env("XDG_CONFIG_HOME", xdg.path().join("config"))
        .env("XDG_DATA_HOME", xdg.path().join("data"))
        .env("XDG_STATE_HOME", xdg.path().join("state"))
        .env("XDG_CACHE_HOME", xdg.path().join("cache"))
        .assert())
}

/// Returns a temporary directory that holds the list `latin` with the bytes
/// [`LATIN_1`].
///
/// # Errors
///
/// Returns the I/O error of a directory or a file that cannot be created.
fn latin_list() -> io::Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes its own list into a temporary directory that it owns"
    )]
    std::fs::write(dir.path().join("latin.txt"), LATIN_1)?;
    Ok(dir)
}

/// Returns the examples of the built `reading` binary with `READING_DIR` set
/// to the fixture lists.
fn examples() -> Examples<'static> {
    Examples::new(env!("CARGO_BIN_EXE_reading"), "").env(&[("READING_DIR", LISTS)])
}

#[test]
fn entries_export_prints_the_file_of_the_list_unchanged() -> io::Result<()> {
    reading(LISTS, &["entries", "export", "--list", "to-read"])?
        .success()
        .stdout_eq(TO_READ)
        .stderr_eq("");
    Ok(())
}

#[test]
fn entries_export_with_json_prints_a_utf_8_file_as_text() -> io::Result<()> {
    let output = reading(LISTS, &["entries", "export", "--list", "to-read", "--json"])?
        .success()
        .stderr_eq("")
        .get_output()
        .stdout
        .clone();

    // The comparison is of the bytes, because snapbox's own comparison
    // reads `\n` in the JSON as a path separator.
    let expected = concat!(
        r##"{"schema":"reading/entries-export/v1","data":{"text":"# Books to read next\nNorth"##,
        r#" and South\nWuthering Heights\nThe Moonstone\nDracula\nLittle Women\n"}}"#,
        "\n",
    );
    assert_eq!(String::from_utf8_lossy(&output), expected);
    Ok(())
}

#[test]
fn entries_export_prints_a_file_that_is_not_utf_8_unchanged() -> io::Result<()> {
    let dir = latin_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["entries", "export", "--list", "latin"])?
        .success()
        .stdout_eq(LATIN_1)
        .stderr_eq("");
    Ok(())
}

#[test]
fn entries_export_with_json_prints_a_file_that_is_not_utf_8_as_base64() -> io::Result<()> {
    let dir = latin_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["entries", "export", "--list", "latin", "--json"])?
        .success()
        .stdout_eq(concat!(
            r#"{"schema":"reading/entries-export/v1","data":{"base64":"#,
            r#""IyBCb29rcyBpbiBMYXRpbi0xCkxlcyBNaXPpcmFibGVzCg=="}}"#,
            "\n",
        ))
        .stderr_eq("");
    Ok(())
}

#[test]
fn entries_export_of_a_list_that_does_not_exist_fails_with_a_next_step() -> io::Result<()> {
    reading(LISTS, &["entries", "export", "--list", "unread"])?
        .code(1)
        .stdout_eq("")
        .stderr_eq(
            "error: reading has no list named unread\n  help:  run `reading commands list`\n  code:  reading::entries::not_found\n",
        );
    Ok(())
}

#[test]
fn entries_export_refuses_fields_and_admits_its_twin_without_them() {
    // The twin leaves out `--fields=text`, so the flag is the reason for the
    // refusal (TEST-1).
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples(),
        &Failing::new(
            &codes::CLI_USAGE,
            &["entries", "export", "--list", "to-read", "--fields=text"],
        ),
        &["entries", "export", "--list", "to-read"],
    );
    OutputAssert::new(refused).code(2).stdout_eq("").stderr_eq(
        "error: --fields selects the columns of a table, and this command prints raw bytes\n  help:  use --json to read the bytes as text or base64\n  code:  reading::cli::usage\n",
    );
}

#[test]
fn entries_export_refuses_long_and_admits_its_twin_without_it() {
    // The twin leaves out `--long`, so the flag is the reason for the
    // refusal (TEST-1).
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples(),
        &Failing::new(
            &codes::CLI_USAGE,
            &["entries", "export", "--list", "to-read", "--long"],
        ),
        &["entries", "export", "--list", "to-read"],
    );
    OutputAssert::new(refused).code(2).stdout_eq("");
}

#[test]
fn entries_export_refuses_full_and_admits_its_twin_with_json_alone() {
    // The twin keeps `--json` and leaves out `--full`, so `--full` is the
    // reason for the refusal (TEST-1).
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples(),
        &Failing::new(
            &codes::CLI_USAGE,
            &["entries", "export", "--list", "to-read", "--json", "--full"],
        ),
        &["entries", "export", "--list", "to-read", "--json"],
    );
    OutputAssert::new(refused).code(2).stderr_eq("").stdout_eq(concat!(
        r#"{"schema":"reading/failure/v1","code":"reading::cli::usage","#,
        r#""message":"--full prints every field of every row, and this command prints raw bytes","#,
        r#""fields":{"argument":"--full"},"recovery":{"kind":"hint","#,
        r#""text":"drop --full, because --json already prints the whole result"}}"#,
        "\n",
    ));
}
