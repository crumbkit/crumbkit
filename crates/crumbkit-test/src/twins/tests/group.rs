//! Tests for a refused line that stops at a group, which prints the
//! group's help page in text and its failure in JSON (ADR-0118).

use crumbkit::Code;
use crumbkit::help::Guide;

use super::tree;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::check_twin;

/// The code of a refused command line.
const USAGE: Code = crumbkit::codes::CLI_USAGE;

/// The script's answers: `books` prints the group's help page and exits
/// with code 2, `books --json` prints a failure document with the usage
/// code, and `books list` succeeds. Any other command line fails with code
/// 3.
const SCRIPT: &str = r#"#!/bin/sh
case "$*" in
  'books') printf 'Usage: tool books <COMMAND>\n\nCommands:\n  list  List books\n' >&2; exit 2;;
  'books --json') printf '%s\n' '{"schema":"tool/failure/v1","code":"tool::cli::usage","message":"m","recovery":{"kind":"retry"}}'; exit 2;;
  'books list') echo listed; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Writes the script and returns its directory and `Examples`' binary path.
fn tool(label: &str) -> (TempDir, String) {
    let dir = TempDir::new(label).unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    (dir, path)
}

#[test]
fn a_refused_line_that_prints_a_group_s_help_page_passes_with_the_code_of_its_json_run() {
    let (_dir, path) = tool("twins-group");
    let examples = Examples::new(&path, "");

    let output = check_twin(
        &tree(),
        &Guide::new(),
        &examples,
        &Failing::new(&USAGE, &["books"]),
        &["books", "list"],
    );

    assert!(String::from_utf8_lossy(&output.stderr).starts_with("Usage: tool books <COMMAND>\n"));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_refused_line_that_prints_a_group_s_help_page_fails_when_its_json_run_prints_another_code() {
    const OTHER: Code = Code::new("tool", "books", "other");
    let (_dir, path) = tool("twins-group-other");
    let examples = Examples::new(&path, "");

    let message = panic_message(|| {
        check_twin(
            &tree(),
            &Guide::new(),
            &examples,
            &Failing::new(&OTHER, &["books"]),
            &["books", "list"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the refused line `tool books` must fail with tool::books::other, and it fails with tool::cli::usage"
    );
}
