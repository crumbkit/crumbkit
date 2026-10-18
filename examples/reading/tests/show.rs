//! End-to-end tests of `reading lists show`, which prints the file of each
//! list it names under a header. Each test runs the built binary with
//! `READING_DIR` set to the fixture lists or to a temporary directory.

use std::io;
use std::path::Path;

use crumbkit::codes;
use crumbkit_test::{Examples, Failing, check_twin};
use snapbox::cmd::{Command, OutputAssert};

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// The text of the to-read fixture list.
const TO_READ: &str = "# Books to read next\nNorth and South\nWuthering Heights\nThe Moonstone\nDracula\nLittle Women\n";

/// The text of the finished fixture list.
const FINISHED: &str = "# Books finished this year\nEmma\nJane Eyre\n";

/// The bytes of a list whose second line is in Latin-1 and so is not
/// UTF-8.
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

#[test]
fn lists_show_prints_the_file_of_each_list_under_a_header_with_a_blank_line_between()
-> io::Result<()> {
    reading(LISTS, &["lists", "show", "to-read", "finished"])?
        .success()
        .stdout_eq(format!(
            "==> to-read <==\n{TO_READ}\n==> finished <==\n{FINISHED}"
        ))
        .stderr_eq("");
    Ok(())
}

#[test]
fn lists_show_of_one_list_prints_its_file_without_a_header() -> io::Result<()> {
    reading(LISTS, &["lists", "show", "finished"])?
        .success()
        .stdout_eq(FINISHED)
        .stderr_eq("");
    Ok(())
}

#[test]
fn lists_show_prints_a_file_that_is_not_utf_8_unchanged_once_for_each_time_it_is_named()
-> io::Result<()> {
    let dir = latin_list()?;
    let dir = dir.path().to_string_lossy();
    let header = b"==> latin <==\n";
    let expected = [&header[..], LATIN_1, b"\n", header, LATIN_1].concat();

    reading(&dir, &["lists", "show", "latin", "latin"])?
        .success()
        .stdout_eq(expected)
        .stderr_eq("");
    Ok(())
}

#[test]
fn lists_show_of_a_list_that_does_not_exist_prints_the_others_and_fails_with_its_key()
-> io::Result<()> {
    reading(LISTS, &["lists", "show", "unread", "to-read"])?
        .code(1)
        .stdout_eq(format!("==> to-read <==\n{TO_READ}"))
        .stderr_eq(
            "error: reading has no list named unread\n  key:   unread (index 0)\n  help:  run `reading commands list`\n  code:  reading::entries::not_found\n",
        );
    Ok(())
}

#[test]
fn lists_show_with_json_prints_each_file_as_text_or_base64_beside_the_list_s_name() -> io::Result<()>
{
    let dir = latin_list()?;
    let dir = dir.path().to_string_lossy();

    reading(&dir, &["lists", "show", "latin", "--json"])?
        .success()
        .stdout_eq(concat!(
            r#"{"schema":"reading/lists-show/v1","data":[{"name":"latin","base64":"#,
            r#""IyBCb29rcyBpbiBMYXRpbi0xCkxlcyBNaXPpcmFibGVzCg=="}],"failures":[]}"#,
            "\n",
        ))
        .stderr_eq("");
    Ok(())
}

#[test]
fn lists_show_refuses_fields_without_json_and_admits_its_twin_with_json() {
    // The twin adds `--json`, so the missing `--json` is the reason for the
    // refusal (TEST-1).
    let examples = Examples::new(env!("CARGO_BIN_EXE_reading"), "").env(&[("READING_DIR", LISTS)]);
    let refused = check_twin(
        &reading::command(&reading::lists_in(Path::new(LISTS))),
        &reading::GUIDE,
        &examples,
        &Failing::new(
            &codes::CLI_USAGE,
            &["lists", "show", "to-read", "--fields=name"],
        ),
        &["lists", "show", "to-read", "--fields=name", "--json"],
    );
    OutputAssert::new(refused).code(2).stdout_eq("").stderr_eq(
        "error: --fields selects the fields of JSON rows, and this command's text prints each answer's bytes\n  help:  add --json, or drop --fields\n  code:  reading::cli::usage\n",
    );
}
