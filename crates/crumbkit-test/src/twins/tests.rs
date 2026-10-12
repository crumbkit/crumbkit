//! Tests for the twin check.
//!
//! Each test writes a `/bin/sh` script that stands in for a tool, as the
//! recovery check's tests do.

use crumbkit::form::{FailureForm, PreparedFailure, ReadFailure};
use crumbkit::{Code, Document, Spelling};
use schemars::{Schema, json_schema};

use super::check_twin;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The code of a refused command line.
const USAGE: Code = crumbkit::codes::CLI_USAGE;

/// The script's answers: `books list --limit 0` fails with the usage code in
/// text and in JSON, `--limit -1` fails with the usage code in the text of
/// `LineForm`, `--limit 1` succeeds, and `--limit 2` exits with code 0 as
/// well. Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
case "$*" in
  'books list --limit 0') printf 'error: 0 is not a limit\n  help:  run `tool books list -h`\n  code:  crumbkit::cli::usage\n' >&2; exit 2;;
  'books list --limit 0 --json') printf '%s\n' '{"schema":"crumbkit/failure/v1","code":"crumbkit::cli::usage","message":"m","recovery":{"kind":"retry"}}'; exit 2;;
  'books list --limit -1') printf 'ERR crumbkit::cli::usage\n' >&2; exit 2;;
  'books list --limit 1'|'books list --limit 1 --json'|'books list --limit 2') echo listed; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Writes the script and returns its directory and path.
fn tool(label: &str) -> (TempDir, String) {
    let dir = TempDir::new(label).unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    (dir, path)
}

#[test]
fn a_twin_passes_when_the_refused_line_fails_with_its_code_and_the_admitted_line_succeeds() {
    let (_dir, path) = tool("twins-pass");
    let examples = Examples::new(&path, "");

    let text = check_twin(
        &examples,
        Spelling::COLONS,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
        &["books", "list", "--limit", "1"],
    );
    let json = check_twin(
        &examples,
        Spelling::COLONS,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0", "--json"]),
        &["books", "list", "--limit", "1", "--json"],
    );

    assert!(String::from_utf8_lossy(&text.stderr).starts_with("error: 0 is not a limit\n"));
    assert!(String::from_utf8_lossy(&json.stdout).contains("\"code\":\"crumbkit::cli::usage\""));
}

#[test]
fn a_refused_line_that_succeeds_or_fails_with_another_code_fails_the_check() {
    const OTHER: Code = Code::new("tool", "books", "not_found");
    let (_dir, path) = tool("twins-refused");
    let examples = Examples::new(&path, "");

    let succeeds = panic_message(|| {
        check_twin(
            &examples,
            Spelling::COLONS,
            &Failing::new(&USAGE, &["books", "list", "--limit", "2"]),
            &["books", "list", "--limit", "1"],
        );
    });
    let other = panic_message(|| {
        check_twin(
            &examples,
            Spelling::COLONS,
            &Failing::new(&OTHER, &["books", "list", "--limit", "0"]),
            &["books", "list", "--limit", "1"],
        );
    });

    assert_eq!(
        succeeds.unwrap(),
        "the refused line `tool books list --limit 2` must fail with crumbkit::cli::usage, and it exits with code 0"
    );
    assert_eq!(
        other.unwrap(),
        "the refused line `tool books list --limit 0` must fail with tool::books::not_found, and it fails with crumbkit::cli::usage"
    );
}

#[test]
fn an_admitted_line_that_fails_fails_the_check() {
    let (_dir, path) = tool("twins-admitted");

    let message = panic_message(|| {
        check_twin(
            &Examples::new(&path, ""),
            Spelling::COLONS,
            &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
            &["books", "list", "--limit", "3"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the admitted line `tool books list --limit 3` must exit with code 0, and it ends with exit status: 3 after writing this to standard error:\nunexpected: books list --limit 3\n"
    );
}

#[test]
fn twins_that_differ_by_more_than_one_word_fail_the_check() {
    let (_dir, path) = tool("twins-words");

    let message = panic_message(|| {
        check_twin(
            &Examples::new(&path, ""),
            Spelling::COLONS,
            &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
            &["books", "list"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the refused line `tool books list --limit 0` and the admitted line `tool books list` must differ by one word added, removed or replaced"
    );
}

/// A failure form whose text is one line, `ERR` and the code, and which
/// reads that line back.
struct LineForm;

impl FailureForm for LineForm {
    fn document(&self) -> Document<()> {
        Document::new("tool", "error", 1, "The document of a failed command.")
    }

    fn schema(&self) -> Schema {
        json_schema!({ "type": "object" })
    }

    fn json(&self, failure: &PreparedFailure<'_>) -> serde_json::Value {
        serde_json::json!({ "error": failure.code() })
    }

    fn read(&self, _out: &str, err: &str) -> Option<ReadFailure> {
        let code = err.lines().last()?.strip_prefix("ERR ")?;
        Some(ReadFailure::new(code.to_owned(), Vec::new()))
    }
}

static FORM: LineForm = LineForm;

#[test]
fn a_twin_reads_the_refused_lines_code_through_the_failure_form_of_examples() {
    let (_dir, path) = tool("twins-form");

    let output = check_twin(
        &Examples::new(&path, "").failure_form(&FORM),
        Spelling::COLONS,
        &Failing::new(&USAGE, &["books", "list", "--limit", "-1"]),
        &["books", "list", "--limit", "1"],
    );

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "ERR crumbkit::cli::usage\n"
    );
}

#[test]
fn a_refused_line_whose_code_the_check_cannot_read_fails_the_check_naming_failure_form_read() {
    let (_dir, path) = tool("twins-unread");

    let message = panic_message(|| {
        check_twin(
            &Examples::new(&path, ""),
            Spelling::COLONS,
            &Failing::new(&USAGE, &["books", "list", "--limit", "-1"]),
            &["books", "list", "--limit", "1"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the refused line `tool books list --limit -1` must fail with crumbkit::cli::usage, and it ends with exit status: 2 and prints no code that the check can read. A tool that prints failures in a form of its own implements FailureForm::read and passes the form to Examples::failure_form"
    );
}

#[test]
fn a_failing_line_without_a_dir_runs_in_the_current_dir_of_examples() {
    let (dir, path) = tool("twins-current-dir");
    dir.write_script(
        "picky",
        "#!/bin/sh\ntest -f marker || exit 3\nexec \"$(dirname \"$0\")/tool\" \"$@\"\n",
    );
    let work = TempDir::new("twins-current-dir-work").unwrap();
    work.write("marker", "");
    let picky = path.replace("/tool", "/picky");
    let work_path = work.path().display().to_string();

    check_twin(
        &Examples::new(&picky, "").current_dir(&work_path),
        Spelling::COLONS,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
        &["books", "list", "--limit", "1"],
    );
}

#[test]
fn both_lines_of_a_twin_run_in_a_copy_of_the_refused_line_s_xdg_fixture() {
    let (dir, path) = tool("twins-xdg");
    dir.write_script(
        "plugged",
        "#!/bin/sh\ntest -f \"$XDG_CONFIG_HOME/tool/plugin\" || exit 3\nexec \"$(dirname \"$0\")/tool\" \"$@\"\n",
    );
    let fixture = TempDir::new("twins-xdg-fixture").unwrap();
    fixture.create_dir("config/tool").unwrap();
    fixture.write("config/tool/plugin", "");
    let plugged = path.replace("/tool", "/plugged");
    let fixture_path = fixture.path().display().to_string();

    check_twin(
        &Examples::new(&plugged, ""),
        Spelling::COLONS,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]).xdg(&fixture_path),
        &["books", "list", "--limit", "1"],
    );
}
